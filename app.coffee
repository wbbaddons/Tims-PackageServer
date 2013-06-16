serverVersion = '1.0.0'

express = require 'express'
fs = require 'fs'
path = require 'path'
async = require 'async'
tar = require 'tar'
xml = 
	parser: (require 'xml2js').Parser
	writer: require 'xml-writer'
logger = new (require 'caterpillar').Logger
	level: 6

((logger.pipe new (require('caterpillar-filter').Filter)).pipe new (require('caterpillar-human').Human)).pipe process.stdout

try
	filename = "#{__dirname}/config.js"
	if process.argv[2]
		if process.argv[2].substring(0, 1) is '/'
			filename = process.argv[2]
		else
			filename = "#{__dirname}/../#{process.argv[2]}"
	
	filename = fs.realpathSync filename
	
	logger.log "info", "Using config '#{filename}'"
	config = require filename
catch e
	logger.log "warn", e.message
	config = { }

config.port ?= 9001
config.packageFolder ?= "#{__dirname}/packages/"
config.packageFolder += '/' unless /\/$/.test config.packageFolder
config.enableManualUpdate = on

app = do express
app.disable 'x-powered-by'
#app.use do express.logger
app.use do express.compress

packageList = { }
updating = no
updateTimeout = null
writer = null
lastUpdate = new Date
updateTime = 0
watcher = [ ]

updateWatcher = ->
	async.each watcher, (obj, callback) ->
		do obj.close
		do callback
	watcher = [ ]
	
	watcher.push fs.watch config.packageFolder, (event, filename) ->
		logger.log "note", "The package folder was changed (#{config.packageFolder}#{filename}: #{event})"
		clearTimeout updateTimeout if updateTimeout?
		updateTimeout = setTimeout readPackages, 1e3
		
	fs.readdir config.packageFolder, (err, files) ->
		async.each files, (file, callback) ->
			watcher.push fs.watch config.packageFolder + file, (event, filename) ->
				logger.log "note", "The package folder was changed (#{config.packageFolder}#{file}/#{filename}: #{event})"
				clearTimeout updateTimeout if updateTimeout?
				updateTimeout = setTimeout readPackages, 1e3
				
getPackageXml = (filename, callback) ->
	stream = fs.createReadStream filename
	tarStream = stream.pipe do tar.Parse
	
	packageXmlFound = no
	
	stream.on 'end', ->
		callback "package.xml is missing in #{filename}", null unless packageXmlFound
	
	tarStream.on 'entry', (e) ->
		return unless e.props.path is 'package.xml'
		packageXmlFound = yes
		packageXml = ''
		e.on 'data', (chunk) ->
			packageXml += do chunk.toString
		e.on 'end', ->
			(new xml.parser()).parseString packageXml, (err, contents) ->
				if err?
					callback "Error parsing package.xml of #{filename}: #{err}", null
				callback null, contents
				
readPackages = (callback) ->
	return if updating
	updating = yes
	updateStart = do (new Date).getTime
	fs.readdir config.packageFolder, (err, files) ->
		logger.log "info", "Starting update"
		if err?
			logger.log "crit", err
			return
		
		newPackageList = { }
		
		async.eachSeries files, (file, fileCallback) ->
			if file is '.gitignore'
				fileCallback null
				return

			packageFolder = config.packageFolder + file
			logger.log "debug", "Parsing #{packageFolder}"
			fs.stat packageFolder, (err, packageFolderStat) ->
				if err?
					fileCallback err
					return
					
				unless do packageFolderStat.isDirectory
					logger.log "warn", "#{packageFolder} is not a folder"
					return
				
				latest = packageFolder + '/latest'
				logger.log "debug", "Parsing #{latest}"
				currentPackage = { }
				parsingFinished = ->
					newPackageList[currentPackage.name] = currentPackage
					fileCallback null
				
				fs.exists latest, (latestExists) ->
					unless latestExists
						logger.log "warn", "#{latest} is missing"
						fileCallback null
						return
						
					getPackageXml latest, (err, latestPackageXml) ->
						if err?
							fileCallback "Error parsing package.xml of #{latest}: #{err}"
							return
						logger.log "debug", "Finished parsing #{latest}"
						currentPackage.name = latestPackageXml.package.$.name
						if currentPackage.name isnt path.basename packageFolder
							fileCallback "package name does not match folder (#{currentPackage.name} != #{path.basename packageFolder})"
							return
							
						currentPackage.packageinformation = latestPackageXml.package.packageinformation[0]
						currentPackage.authorinformation = latestPackageXml.package.authorinformation[0]
						currentPackage.versions = { }
						
						fs.readdir packageFolder, (err, versions) ->
							if err?
								fileCallback err
								return
							async.eachSeries versions, (versionFile, versionsCallback) ->
								if versionFile is 'latest'
									versionsCallback null
									return
									
								versionFile = packageFolder + '/' + versionFile
								logger.log "debug", "Parsing #{versionFile}"
								fs.stat versionFile, (err, versionFileStat) ->
									if err?
										versionsCallback versionFileStat
										return
								
									unless do versionFileStat.isFile
										logger.log "warn", "#{versionFile} is not a file"
										return
										
									getPackageXml versionFile, (err, versionPackageXml) ->
										if err?
											versionsCallback err
											return
										versionNumber = versionPackageXml.package.packageinformation[0].version[0]
										currentVersion = { }
										currentVersion.versionnumber = versionNumber
										currentVersion.license = versionPackageXml.package.packageinformation[0].license?[0]
										if (currentVersion.versionnumber.replace new RegExp(' ', 'g'), '_') isnt path.basename versionFile, '.tar'
											fileCallback "version number does not match file (#{currentVersion.versionnumber.replace ' ', '_'} != #{path.basename versionFile, '.tar'})"
											return
										currentVersion.fromversions = (instruction.$.fromversion for instruction in versionPackageXml.package.instructions when instruction.$.fromversion?)
										currentVersion.requiredpackages = versionPackageXml.package.requiredpackages[0]
										currentVersion.timestamp = versionFileStat.mtime
										currentPackage.versions[versionNumber] = currentVersion
										# TODO: Excludes and optionals
										logger.log "debug", "Finished parsing #{versionFile}"
										versionsCallback null
							, (err) ->
								if err?
									fileCallback err
									return
								do parsingFinished
		, (err) ->
			do updateWatcher
			if err?
				logger.log "crit", "Error reading package list: #{err}"
				updating = no
				callback false if callback?
				return
			
			packageList = newPackageList
			writer = null
			lastUpdate = new Date
			updateTime = ((do lastUpdate.getTime) - updateStart) / 1e3
			logger.log "info", "Finished update"
			updating = no
			(callback true) if callback?
			
app.all '/', (req, res) ->
	req.accepts 'xml'
	res.type 'xml'
	
	unless writer?
		start = do (new Date).getTime
		writer = new xml.writer true
		writer.startDocument '1.0', 'UTF-8'
		writer.startElement 'section'
		writer.writeAttribute 'name', 'packages'
		writer.writeAttribute 'xmlns', 'http://www.woltlab.com'
		writer.writeAttributeNS 'xmlns', 'xsi', 'http://www.w3.org/2001/XMLSchema-instance'
		writer.writeAttributeNS 'xsi', 'schemaLocation', 'http://www.woltlab.com https://www.woltlab.com/XSD/packageUpdateServer.xsd'
		for packageName, _package of packageList
			writer.startElement 'package'
			writer.writeAttribute 'name', _package.name
			writer.startElement 'packageinformation'
			writer.writeElement 'packagename', _package.packageinformation.packagename[0]._ ? _package.packageinformation.packagename[0]
			writer.writeElement 'packagedescription', _package.packageinformation.packagedescription[0]._ ? _package.packageinformation.packagedescription[0]
			writer.writeElement 'isapplication', String(_package.packageinformation.isapplication ? 0)
			do writer.endElement
			if _package.authorinformation?
				writer.startElement 'authorinformation'
				writer.writeElement 'author', _package.authorinformation.author[0] if _package.authorinformation.author[0]?
				writer.writeElement 'authorurl', _package.authorinformation.authorurl[0] if _package.authorinformation.authorurl[0]?
				do writer.endElement
			writer.startElement 'versions'
			for versionNumber, version of _package.versions
				writer.startElement 'version'
				writer.writeAttribute 'name', versionNumber
				writer.writeAttribute 'accessible', "true"
				writer.writeAttribute 'isCritical', if (/pl/i.test versionNumber) then "true" else "false"
				if version.fromversions.length
					writer.startElement 'fromversions'
					for fromVersion in version.fromversions
						writer.writeElement 'fromversion', fromVersion
					do writer.endElement
				if version.requiredpackages.requiredpackage.length
					writer.startElement 'requiredpackages'
					for requiredPackage in version.requiredpackages.requiredpackage
						writer.startElement 'requiredpackage'
						writer.writeAttribute 'minversion', requiredPackage.$.minversion if requiredPackage.$.minversion?
						writer.text requiredPackage._
						do writer.endElement
					do writer.endElement
				writer.writeElement 'timestamp', String(Math.floor (do version.timestamp.getTime) / 1000)
				writer.writeElement 'file', "{{packageServerHost}}/#{_package.name}/#{versionNumber.replace(new RegExp(' ', 'g'), '_')}"
				if version.license
					result = /^(.*?)(?:\s<(https?:\/\/.*)>)?$/.exec version.license
					writer.startElement 'license'
					writer.writeAttribute 'url', result[2] if result[2]?
					writer.text result[1]
					do writer.endElement
				do writer.endElement
			do writer.endElement
			do writer.endElement
		end = do (new Date).getTime
		writer.writeComment "xml generated in #{(end - start) / 1e3} seconds"
		writer.writeComment "packages scanned in #{updateTime} seconds"
		writer.writeComment "last update #{lastUpdate}"
		writer.writeComment "This list was presented by Tims Package Server #{serverVersion}"
		do writer.endElement
		do writer.endDocument
	res.end (do writer.toString).replace /\{\{packageServerHost\}\}/g, "#{req.protocol}://#{req.header 'host'}"
	
app.all /^\/([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)\/([0-9]+\.[0-9]+\.[0-9]+(?:_(?:a|alpha|b|beta|d|dev|rc|pl)_[0-9]+)?)/i, (req, res) ->
	res.attachment "#{req.params[0]}_#{req.params[1]}.tar"
	res.sendfile "#{config.packageFolder}/#{req.params[0]}/#{req.params[1]}.tar"

if config.enableManualUpdate
	app.get '/update', (req, res) ->
		logger.log "info", 'Manual update was requested'
		readPackages ->
			res.redirect 303, '/'

readPackages ->
	app.listen config.port
	