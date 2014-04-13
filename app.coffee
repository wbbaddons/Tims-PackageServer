###
Copyright (c) 2013 - 2014, Tim Düsterhus
All rights reserved.

Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

- Redistributions of source code must retain the above copyright notice, this list
  of conditions and the following disclaimer.

- Redistributions in binary form must reproduce the above copyright notice, this
  list of conditions and the following disclaimer in the documentation and/or
  other materials provided with the distribution.

- Neither the name of wbbaddons nor the names of its contributors may be used to
  endorse or promote products derived from this software without specific prior
  written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE
###

serverVersion = '1.0.0'

express = require 'express'
fs = require 'fs'
path = require 'path'
async = require 'async'
tar = require 'tar'
bcrypt = require 'bcrypt'
watchr = require 'watchr'

xml = 
	parser: (require 'xml2js').Parser
	writer: require 'xml-writer'
logger = new (require 'caterpillar').Logger
	level: 6

((logger.pipe new (require('caterpillar-filter').Filter)).pipe new (require('caterpillar-human').Human)).pipe process.stdout

# Try to load config
try
	filename = "#{__dirname}/config.js"

	# configuration file was passed via `process.argv`
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

# default values for configuration
config.port ?= 9001
config.ip ?= '0.0.0.0'
config.packageFolder ?= "#{__dirname}/packages/"
config.packageFolder += '/' unless /\/$/.test config.packageFolder
config.enableManualUpdate = on
config.enableStatistics = on

# initialize express
app = do express

# don't tell anyone we are running express
app.disable 'x-powered-by'

#app.use do express.logger
app.use do express.compress

packageList = { }
updating = no
updateTimeout = null
lastUpdate = new Date
updateTime = 0
watcher = [ ]
auth = null
downloadCounterFiles = { }

logDownload = (packageName, version) ->
	return unless config.enableStatistics
	
	version = version.toLowerCase().replace (new RegExp ' ', 'g'), '_'
	unless downloadCounterFiles[packageName]?[version]?
		downloadCounterFiles[packageName] = { } unless downloadCounterFiles[packageName]?
		downloadCounterFiles[packageName][version] = 
			ready: no
			downloads: 0
		
		fs.readFile "#{config.packageFolder}#{packageName}/#{version}.txt", (err, data) ->
			try
				if err
					downloadCounterFiles[packageName][version].ready = yes
					return
					
				downloads = parseInt data.toString()
				downloadCounterFiles[packageName][version].downloads += downloads
				downloadCounterFiles[packageName][version].ready = yes
			finally
				logDownload packageName, version
		return
		
	fs.writeFile "#{config.packageFolder}#{packageName}/#{version}.txt", ++downloadCounterFiles[packageName][version].downloads
	
# Creates watchers for every relevant file in the package folder
do ->
	watchr.watch
		path: config.packageFolder
		ignoreCustomPatterns: /\.txt$/
		listeners:
			watching: (err,watcherInstance,isWatching) ->
				if err
					console.log("watching the path " + watcherInstance.path + " failed with error", err);
				else
					console.log("watching the path " + watcherInstance.path + " completed");
			change: (changeType, filePath, fileCurrentStat, filePreviousStat) ->
				logger.log "notice", "The package folder was changed (#{filePath}: #{changeType})"
				clearTimeout updateTimeout if updateTimeout?
				updateTimeout = setTimeout readPackages, 1e3

createComparator = (comparison) ->
	# simply return true if versions are *
	(return -> true) if comparison is '*'
	
	# normalize comparison string
	comparison = comparison.replace /([0-9]+\.[0-9]+\.[0-9]+(?:([ _])(?:a|alpha|b|beta|d|dev|rc|pl)([ _])[0-9]+)?)/ig, (version) ->
		version = version.replace(/[ _]/g, '.').replace(/a(?:lpha)/i, -3).replace(/b(?:eta)?/i, -2).replace(/d(?:ev)?/i, -4).replace(/rc/i, -1).replace(/pl/i, 1).split(/\./)
		version[0] ?= 0
		version[1] ?= 0
		version[2] ?= 0
		version[3] ?= 0
		version[4] ?= 0
		version = version.join '.'
		
	comparison = comparison.replace /[ ]/g, ''
	comparison = comparison.replace /\$v(==|<=|>=|<|>)([0-9]+\.[0-9]+\.[0-9]+.-?[0-9]+.[0-9]+)/g, (comparison, operator, v2) ->
		# build comparison function
		return """
		((function($v) {
			$v = $v.replace(/[ _]/g, '.').replace(/a(?:lpha)/i, -3).replace(/b(?:eta)?/i, -2).replace(/d(?:ev)?/i, -4).replace(/rc/i, -1).replace(/pl/i, 1).split(/\\./)
			if ($v[0] == null) $v[0] = 0
			if ($v[1] == null) $v[1] = 0
			if ($v[2] == null) $v[2] = 0
			if ($v[3] == null) $v[3] = 0
			if ($v[4] == null) $v[4] = 0
			v2 = "#{v2}".split(/\\./)
			
			result = 0
			for (var i = 0; i < 5; i++) {
				if (parseInt($v[i]) == parseInt(v2[i])) continue;
				if (parseInt($v[i]) < parseInt(v2[i])) {
					result = -1;
					break;
				}
				if (parseInt($v[i]) > parseInt(v2[i])) {
					result = 1;
					break;
				}
			}
			return result #{operator} 0;
		})($v))"""
	new Function '$v', 'return ' + comparison
	
isAccessible = (username, testPackage, testVersion) ->
	return true if auth is null
	
	# check general packages
	if auth.packages?
		for _package, version of auth.packages
			_package = _package.replace(/([.?*+^$[\]\\(){}|-])/g, "\\$1").replace(/\\\*/, '.*')
			return true if (RegExp("^#{_package}$", 'i').test testPackage) and version testVersion
	
	# check user
	if auth.users?[username]?.packages?
		for _package, version of auth.users[username].packages
			_package = _package.replace(/([.?*+^$[\]\\(){}|-])/g, "\\$1").replace(/\\\*/, '.*')
			return true if (RegExp("^#{_package}$", 'i').test testPackage) and version testVersion
		# check user groups
		for group in auth.users[username].groups
			if auth.groups?[group]?
				for _package, version of auth.groups[group]
					_package = _package.replace(/([.?*+^$[\]\\(){}|-])/g, "\\$1").replace(/\\\*/, '.*')
					return true if (RegExp("^#{_package}$", 'i').test testPackage) and version testVersion
	false

# extracts and parses the package.xml of the archive given as `filename`
getPackageXml = (filename, callback) ->
	stream = fs.createReadStream filename
	tarStream = stream.pipe do tar.Parse
	
	packageXmlFound = no
	
	# no more data, but no package.xml was found
	stream.on 'end', -> callback "package.xml is missing in #{filename}", null unless packageXmlFound
	
	tarStream.on 'entry', (e) ->
		# we are searching for the package.xml
		return unless e.props.path is 'package.xml'
		
		packageXmlFound = yes
		packageXml = ''
		e.on 'data', (chunk) -> packageXml += do chunk.toString
		e.on 'end', ->
			# we received the full package.xml -> parse it
			(new xml.parser()).parseString packageXml, (err, contents) ->
				if err?
					callback "Error parsing package.xml of #{filename}: #{err}", null
					return
					
				# push the parsed contents to the callback
				callback null, contents
	
	tarStream.on 'error', -> callback "Error while extracting #{filename}", null

# Updates package list.
readPackages = (callback) ->
	return if updating
	updating = yes
	updateStart = do process.hrtime
	
	fs.exists "#{config.packageFolder}auth.json", (authExists) ->
		# no auth.json was found
		unless authExists
			logger.log "warn", "auth.json does not exist. Assuming all packages are free"
			auth = null
			return
			
		fs.readFile "#{config.packageFolder}auth.json", (err, contents) ->
			if err?
				logger.log "error", "error reading auth.json. Denying access to all packages: #{err}"
				auth = { }
				return
			try
				auth = JSON.parse contents
				
				# convert into functions
				if auth.packages?
					auth.packages[_package] = createComparator versions for _package, versions of auth.packages
				if auth.groups?
					(auth.groups[group][_package] = createComparator versions for _package, versions of packages) for group, packages of auth.groups
				if auth.users?
					(auth.users[user].packages[_package] = createComparator versions for _package, versions of auth.users[user].packages) for user of auth.users
				
				logger.log "notice", "Updated auth"
			catch err
				logger.log "error", "error parsing auth.json. Denying access to all packages: #{err}"
				auth = {}
	
	fs.readdir config.packageFolder, (err, files) ->
		logger.log "info", "Starting update"
		if err?
			logger.log "crit", err
			return
		
		newPackageList = { }
		
		# loop over each file in the package folder
		async.eachSeries files, (file, fileCallback) ->
			# ignore dotfiles…
			if (file.substring 0, 1) is '.'
				logger.log "notice", "Skipping dotfile #{config.packageFolder}#{file}"
				fileCallback null
				return
			# … auth.json
			if file is 'auth.json'
				fileCallback null
				return
			# … and files that don't look like a valid package identifier
			unless /^([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)$/i.test file
				logger.log "notice", "Skipping #{config.packageFolder}#{file}, as it does not match a valid package identifier"
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
					do fileCallback
					return
				
				latest = packageFolder + '/latest'
				logger.log "debug", "Parsing #{latest}"
				currentPackage = { }
				
				parsingFinished = ->
					newPackageList[currentPackage.name] = currentPackage
					fileCallback null
				
				fs.exists latest, (latestExists) ->
					# abort if `latest` is missing
					unless latestExists
						logger.log "warn", "#{latest} is missing"
						fileCallback null
						return
					
					# parse package.xml of `latest`
					getPackageXml latest, (err, latestPackageXml) ->
						if err?
							fileCallback "Error parsing package.xml of #{latest}: #{err}"
							return
						logger.log "debug", "Finished parsing #{latest}"
						currentPackage.name = latestPackageXml.package.$.name
						# either latest does not belong here, or the folder is incorrectly named
						unless currentPackage.name is path.basename packageFolder
							fileCallback "package name does not match folder (#{currentPackage.name} isnt #{path.basename packageFolder})"
							return
						
						# provide relevant information to the callback
						currentPackage.packageinformation = latestPackageXml.package.packageinformation[0]
						currentPackage.authorinformation = latestPackageXml.package.authorinformation[0]
						currentPackage.versions = { }
						
						# read every file in the folder to find the available versions
						fs.readdir packageFolder, (err, versions) ->
							if err?
								fileCallback err
								return
							
							versions = versions.filter (versionFile) ->
								if versionFile is 'latest'
									false
								else if (versionFile.substring 0, 1) is '.'
									logger.log "notice", "Skipping dotfile #{packageFolder}/#{versionFile}"
									false
								else if /^([0-9]+\.[0-9]+\.[0-9]+(?:_(?:a|alpha|b|beta|d|dev|rc|pl)_[0-9]+)?)\.txt$/i.test versionFile
									false
								else unless /^([0-9]+\.[0-9]+\.[0-9]+(?:_(?:a|alpha|b|beta|d|dev|rc|pl)_[0-9]+)?)\.tar$/i.test versionFile
									logger.log "notice", "Skipping #{packageFolder}/#{versionFile}, as it does not match a valid version number"
									false
								else
									true
							versions.sort (a, b) ->
								return 0 if a is b
								return 1 if (createComparator "$v > #{b.replace(/\.tar$/, '')}")(a.replace(/\.tar$/, ''))
								return -1
							async.eachSeries versions, (versionFile, versionsCallback) ->
								versionFile = packageFolder + '/' + versionFile
								logger.log "debug", "Parsing #{versionFile}"
								fs.stat versionFile, (err, versionFileStat) ->
									if err?
										versionsCallback versionFileStat
										return
									
									unless do versionFileStat.isFile
										logger.log "warn", "#{versionFile} is not a file"
										return
									
									# parse package.xml
									getPackageXml versionFile, (err, versionPackageXml) ->
										if err?
											versionsCallback err
											return
										versionNumber = versionPackageXml.package.packageinformation[0].version[0]
										# the tar file is incorrectly named -> abort
										if (versionNumber.toLowerCase().replace (new RegExp ' ', 'g'), '_') isnt path.basename versionFile, '.tar'
											fileCallback "version number does not match file (#{versionNumber.toLowerCase().replace new RegExp(' ', 'g'), '_'} != #{path.basename versionFile, '.tar'})"
											return
										
										currentVersion = { }
										currentVersion.versionnumber = versionNumber
										currentVersion.license = versionPackageXml.package.packageinformation[0].license?[0]
										currentVersion.fromversions = (instruction.$.fromversion for instruction in versionPackageXml.package.instructions when instruction.$.fromversion?)
										currentVersion.optionalpackages = versionPackageXml.package?.optionalpackages?[0]
										currentVersion.requiredpackages = versionPackageXml.package?.requiredpackages?[0]
										currentVersion.excludedpackages = versionPackageXml.package?.excludedpackages?[0]
										currentVersion.timestamp = versionFileStat.mtime
										currentPackage.versions[versionNumber] = currentVersion
										
										logger.log "debug", "Finished parsing #{versionFile}"
										versionsCallback null
							, (err) ->
								if err?
									fileCallback err
								else
									do parsingFinished
		, (err) ->
			if err?
				logger.log "crit", "Error reading package list: #{err}"
				updating = no
				callback false if callback?
				return
			
			# overwrite packageList once everything succeeded
			packageList = newPackageList
			# update scan time and statistics
			lastUpdate = new Date
			updateTime = process.hrtime updateStart
			logger.log "info", "Finished update"
			updating = no
			
			# and finally call the callback
			(callback true) if callback?
			
askForCredentials = (req, res) ->
	res.type 'txt'
	res.setHeader 'WWW-Authenticate', 'Basic realm="Please provide proper username and password to access this package"'
	res.send 401, 'Please provide proper username and password to access this package'
	
app.all '/', (req, res) ->
	callback = (username) ->
		host = config.basePath ? "#{req.protocol}://#{req.header 'host'}"
		
		# redirect when ?packageName=com.example.wcf.test[&packageVersion=1.0.0_Alpha_15] was requested
		if req.query?.packageName?
			if req.query.packageVersion?
				res.redirect 301, "#{host}/#{req.query.packageName}/#{req.query.packageVersion.replace (new RegExp ' ', 'g'), '_'}"
			else
				res.redirect 301, "#{host}/#{req.query.packageName}"
			return
		
		req.accepts 'xml'
		res.type 'xml'
		
		# build the xml structure of the package list
		start = do process.hrtime
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
			
			# write <authorinformation>
			if _package.authorinformation?
				writer.startElement 'authorinformation'
				writer.writeElement 'author', _package.authorinformation.author[0] if _package.authorinformation.author[0]?
				writer.writeElement 'authorurl', _package.authorinformation.authorurl[0] if _package.authorinformation.authorurl[0]?
				do writer.endElement
			writer.startElement 'versions'
			for versionNumber, version of _package.versions
				writer.startElement 'version'
				writer.writeAttribute 'name', versionNumber
				
				writer.writeAttribute 'accessible', if (isAccessible username, _package.name, versionNumber) then "true" else "false"
				writer.writeAttribute 'critical', if (/pl/i.test versionNumber) then "true" else "false"
				
				# write <fromversions>
				if version.fromversions?.length
					writer.startElement 'fromversions'
					for fromVersion in version.fromversions
						writer.writeElement 'fromversion', fromVersion
					do writer.endElement
				
				# write <optionalpackages>
				if version.optionalpackages?.optionalpackage?.length
					writer.startElement 'optionalpackages'
					for optionalpackage in version.optionalpackages.optionalpackage
						writer.startElement 'optionalpackage'
						writer.text optionalpackage._ ? optionalpackage
						do writer.endElement
					do writer.endElement
				
				# write <requiredpackages>
				if version.requiredpackages?.requiredpackage?.length
					writer.startElement 'requiredpackages'
					for requiredPackage in version.requiredpackages.requiredpackage
						writer.startElement 'requiredpackage'
						writer.writeAttribute 'minversion', requiredPackage.$.minversion if requiredPackage.$?.minversion?
						writer.text requiredPackage._ ? requiredPackage
						do writer.endElement
					do writer.endElement
				
				# write <excludedpackages>
				if version.excludedpackages?.excludedpackage?.length
					writer.startElement 'excludedpackages'
					for excludedPackage in version.excludedpackages.excludedpackage
						writer.startElement 'excludedpackage'
						writer.writeAttribute 'version', excludedPackage.$.version if excludedPackage.$?.version?
						writer.text excludedPackage._ ? excludedPackage
						do writer.endElement
					do writer.endElement
				
				writer.writeElement 'timestamp', String(Math.floor (do version.timestamp.getTime) / 1000)
				
				# e.g. {{packageServerHost}}/com.example.wcf.test/1.0.0_Alpha_15
				writer.writeElement 'file', "{{packageServerHost}}/#{_package.name}/#{versionNumber.toLowerCase().replace (new RegExp ' ', 'g'), '_'}"
				
				# try to extract license
				if version.license
					result = /^(.*?)(?:\s<(https?:\/\/.*)>)?$/.exec version.license
					writer.startElement 'license'
					writer.writeAttribute 'url', result[2] if result[2]?
					writer.text result[1]
					do writer.endElement
					
				do writer.endElement
			do writer.endElement
			do writer.endElement
		diff = process.hrtime start
		writer.writeComment "xml generated in #{diff[0] + diff[1] / 1e9} seconds"
		writer.writeComment "packages scanned in #{updateTime[0] + updateTime[1] / 1e9} seconds"
		writer.writeComment "last update #{lastUpdate}"
		writer.writeComment "up since #{do process.uptime} seconds"
		writer.writeComment "This list was presented by Tims Package Server #{serverVersion} <https://github.com/wbbaddons/Tims-PackageServer>"
		do writer.endElement
		do writer.endDocument
		res.send 200, (do writer.toString).replace /\{\{packageServerHost\}\}/g, host
	
	if req.auth?
		if auth?.users?[req.auth.username]?
			bcrypt.compare req.auth.password, auth.users[req.auth.username].passwd, (err, result) ->
				if err?
					res.send 500, '500 Internal Server Error'
				else if result
					callback req.auth.username
				else
					askForCredentials req, res
		else
			askForCredentials req, res
	else
		callback ''

# package download requested
app.all /^\/([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)\/([0-9]+\.[0-9]+\.[0-9]+(?:_(?:a|alpha|b|beta|d|dev|rc|pl)_[0-9]+)?)\/?(?:\?.*)?$/i, (req, res) ->
	callback = (username) ->
		fs.exists "#{config.packageFolder}/#{req.params[0]}/#{req.params[1].toLowerCase()}.tar", (packageExists) ->
			if packageExists
				if isAccessible username, req.params[0], req.params[1]
					logger.log "notice", "#{username} downloaded #{req.params[0]}/#{req.params[1].toLowerCase()}"
					logDownload req.params[0], req.params[1]
					res.attachment "#{req.params[0]}_v#{req.params[1]}.tar"
					res.sendfile "#{config.packageFolder}/#{req.params[0]}/#{req.params[1].toLowerCase()}.tar", (err) -> res.send 404, '404 Not Found' if err?
				else
					logger.log "notice", "#{username} tried to download #{req.params[0]}/#{req.params[1].toLowerCase()}"
					askForCredentials req, res
			else
				res.send 404, '404 Not Found'
		
	if req.auth?
		if auth?.users?[req.auth.username]?
			bcrypt.compare req.auth.password, auth.users[req.auth.username].passwd, (err, result) ->
				if err?
					res.send 500, '500 Internal Server Error'
					return
				if result
					callback req.auth.username
				else
					askForCredentials req, res
		else
			askForCredentials req, res
	else
		callback ''

# allow download without version number
app.all /^\/([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)\/?(?:\?.*)?$/i, (req, res) ->
	host = config.basePath ? "#{req.protocol}://#{req.header 'host'}"
	versionNumber = packageList?[req.params[0]]?.packageinformation?.version[0]
	unless versionNumber?
		res.send 404, '404 Not Found'
		return
	res.redirect 301, "#{host}/#{req.params[0]}/#{versionNumber.toLowerCase().replace (new RegExp ' ', 'g'), '_'}"

# manual update via {{packageServerHost}}/update
if config.enableManualUpdate
	app.get '/update', (req, res) ->
		logger.log "info", 'Manual update was requested'
		readPackages -> res.redirect 303, config.basePath ? "#{req.protocol}://#{req.header 'host'}/"

# throw 404 on any unknown route
app.all '*', (req, res) -> res.send 404, '404 Not Found'

# Once the package list was successfully scanned once bind to the port
readPackages -> app.listen config.port, config.ip
