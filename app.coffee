###
Copyright (C) 2013 - 2014 Tim Düsterhus

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
###

panic = -> throw new Error "Cowardly refusing to keep the process alive as root"
panic() if process.getuid?() is 0 or process.getgid?() is 0
	
serverVersion = (require './package.json').version
(require 'child_process').exec 'git describe --always', (err, stdout, stderr) -> serverVersion = stdout.trim() unless err?

async = require 'async'
basicAuth = require 'basic-auth'
bcrypt = require 'bcrypt'
crypto = require 'crypto'
escapeRegExp = require 'escape-string-regexp'
express = require 'express'
expresshb  = require 'express-handlebars'

fs = require 'fs'
path = require 'path'
tarstream = require 'tar-stream'
watchr = require 'watchr'
xmlstream = require 'xml-stream'
xmlwriter = require 'xml-writer'

debug = (require 'debug')('PackageServer:debug')
warn = (require 'debug')('PackageServer:warn')
warn.log = console.warn.bind console
error = (require 'debug')('PackageServer:error')
error.log = console.error.bind console

# Try to load config
try
	filename = "#{__dirname}/config.js"

	# configuration file was passed via `process.argv`
	filename = (require 'path').resolve process.argv[2] if process.argv[2]?
	
	filename = fs.realpathSync filename
	
	debug "Using config '#{filename}'"
	config = require filename
catch e
	warn e.message
	config = { }

# default values for configuration
config.port ?= 9001
config.ip ?= '0.0.0.0'
config.packageFolder ?= "#{__dirname}/packages/"
config.packageFolder += '/' unless /\/$/.test config.packageFolder
config.enableStatistics ?= on
config.enableHash ?= on
config.deterministic ?= off
config.lang ?= {}

if config.enableManualUpdate?
	warn 'config.enableManualUpdate is obsolete and ignored in this version'

# initialize express
app = do express

app.engine 'handlebars', expresshb()
app.set 'view engine', 'handlebars'
# app.enable 'view cache'

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
	
	version = version.toLowerCase().replace /[ ]/g, '_'
	unless downloadCounterFiles[packageName]?[version]?
		downloadCounterFiles[packageName] = { } unless downloadCounterFiles[packageName]?
		downloadCounterFiles[packageName][version] = 0
		
		fs.readFile "#{config.packageFolder}#{packageName}/#{version}.txt", (err, data) ->
			try
				if err
					return
					
				downloads = parseInt data.toString()
				downloadCounterFiles[packageName][version] += downloads
			finally
				logDownload packageName, version
		return
		
	fs.writeFile "#{config.packageFolder}#{packageName}/#{version}.txt", ++downloadCounterFiles[packageName][version]

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
	
	comparatorHelper = ($v, v2) ->
		v2 = v2.split /\./
		$v = $v.replace(/[ _]/g, '.').replace(/a(?:lpha)/i, -3).replace(/b(?:eta)?/i, -2).replace(/d(?:ev)?/i, -4).replace(/rc/i, -1).replace(/pl/i, 1).split(/\./)
		$v[0] ?= 0
		$v[1] ?= 0
		$v[2] ?= 0
		$v[3] ?= 0
		$v[4] ?= 0
		
		result = 0
		for i in [0...5]
			continue if (parseInt $v[i]) is parseInt(v2[i])
			if (parseInt $v[i]) < parseInt(v2[i])
				result = -1
				break
			if (parseInt $v[i]) > parseInt(v2[i])
				result = 1
				break
		result
		
	comparison = comparison.replace /\$v(==|<=|>=|<|>)([0-9]+\.[0-9]+\.[0-9]+.-?[0-9]+.[0-9]+)/g, (comparison, operator, v2) -> """(comparatorHelper($v, "#{v2}") #{operator} 0)"""
	comparison = comparison.replace /\$v~(\/(?:[^/\\]|\\.)+\/i?)/g, (comparison, regex) -> """(#{regex}.test($v))"""
	comparison = comparison.replace /\$v!~(\/(?:[^/\\]|\\.)+\/i?)/g, (comparison, regex) -> """(!#{regex}.test($v))"""
	
	comparator = new Function '$v', 'comparatorHelper', 'return ' + comparison
	
	($v) ->	comparator $v, comparatorHelper

isAccessible = (username, testPackage, testVersion) ->
	return true if auth is null
	
	# check general packages
	if auth.packages?
		for _package, version of auth.packages
			_package = escapeRegExp(_package).replace /\\\*/, '.*'
			return true if (RegExp("^#{_package}$", 'i').test testPackage) and version testVersion
	
	# check user
	if auth.users?[username]?.packages?
		for _package, version of auth.users[username].packages
			_package = escapeRegExp(_package).replace /\\\*/, '.*'
			return true if (RegExp("^#{_package}$", 'i').test testPackage) and version testVersion
		# check user groups
		for group in auth.users[username].groups
			if auth.groups?[group]?
				for _package, version of auth.groups[group]
					_package = escapeRegExp(_package).replace /\\\*/, '.*'
					return true if (RegExp("^#{_package}$", 'i').test testPackage) and version testVersion
	false

checkAuth = (req, res, callback) ->
	reqAuth = basicAuth req
	
	if reqAuth?
		if auth?.users?[reqAuth.name]?
			# hash first because Woltlab Community Framework uses double salted hashes
			bcrypt.hash reqAuth.pass, auth.users[reqAuth.name].passwd, (err, hash) ->
				if err?
					res.sendStatus 500
					return
				bcrypt.compare hash, auth.users[reqAuth.name].passwd, (err, result) ->
					if err?
						res.sendStatus 500
						return
					if result
						callback reqAuth.name
					else
						askForCredentials req, res
		else
			askForCredentials req, res
	else
		callback ''

hashStream = (stream, callback) ->
	hasher = new crypto.Hash 'sha256'
	stream.pipe hasher
	stream.on 'end', ->
		hash = do hasher.read
		debug "Hashed stream: #{hash.toString 'hex'}"
		callback null, hash

# Updates package list.
readPackages = (callback) ->
	updateStart = do process.hrtime
	
	fs.exists "#{config.packageFolder}auth.json", (authExists) ->
		# no auth.json was found
		unless authExists
			warn "auth.json does not exist. Assuming all packages are free"
			auth = null
			return
			
		fs.readFile "#{config.packageFolder}auth.json", (err, contents) ->
			if err?
				error "error reading auth.json. Denying access to all packages: #{err}"
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
					(auth.users[username].packages[_package] = createComparator versions for _package, versions of userdata.packages) for username, userdata of auth.users when userdata.packages?
				
				debug "Updated auth"
			catch err
				error "error parsing auth.json. Denying access to all packages: #{err}"
				auth = { }
	
	fs.readdir config.packageFolder, (err, files) ->
		debug "Starting update"
		if err?
			error err
			return
		
		# ignore dotfiles, auth.json and invalid package identifiers
		files = files.filter (file) ->
			if file[0] is '.'
				debug "Skipping dotfile #{config.packageFolder}#{file}"
				false
			else if file in [ 'auth.json', 'auth.json.example' ]
				false
			else unless /^([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)$/i.test file
				debug "Skipping #{config.packageFolder}#{file}, as it does not match a valid package identifier"
				false
			else
				true
		
		# loop over each file in the package folder
		async.map files, (file, fileCallback) ->
			packageFolder = config.packageFolder + file
			debug "Parsing #{packageFolder}"
			fs.stat packageFolder, (err, packageFolderStat) ->
				if err?
					fileCallback err
					return
					
				unless do packageFolderStat.isDirectory
					fileCallback "#{packageFolder} is not a folder"
					return
				
				# read every file in the folder to find the available versions
				fs.readdir packageFolder, (err, versions) ->
					if err?
						fileCallback err
						return
					
					versions = versions.filter (versionFile) ->
						if versionFile is 'latest'
							warn "The latest symlink is obsolete now. The information are taken from the newest package automatically!"
							false
						else if (versionFile.substring 0, 1) is '.'
							debug "Skipping dotfile #{packageFolder}/#{versionFile}"
							false
						else if /^([0-9]+\.[0-9]+\.[0-9]+(?:_(?:a|alpha|b|beta|d|dev|rc|pl)_[0-9]+)?)\.txt$/i.test versionFile
							false
						else unless /^([0-9]+\.[0-9]+\.[0-9]+(?:_(?:a|alpha|b|beta|d|dev|rc|pl)_[0-9]+)?)\.tar$/i.test versionFile
							debug "Skipping #{packageFolder}/#{versionFile}, as it does not match a valid version number"
							false
						else
							true
					
					async.map versions, (versionFile, versionsCallback) ->
						versionFile = packageFolder + '/' + versionFile
						debug "Parsing #{versionFile}"
						fs.stat versionFile, (err, versionFileStat) ->
							debug "Got #{versionFile} stat"
							if err?
								versionsCallback err
								return
							
							unless do versionFileStat.isFile
								versionsCallback "#{versionFile} is not a file"
								return
							
							archiveStream = fs.createReadStream versionFile
							tasks = [ ]
							
							if config.enableHash
								tasks.push (callback) -> hashStream archiveStream, callback
							
							tasks.unshift (callback) ->
								extract = tarstream.extract()
								
								packageXmlFound = no
								
								# no more data, but no package.xml was found
								extract.on 'finish', -> callback "package.xml is missing in #{versionFile}" unless packageXmlFound
								
								extract.on 'entry', (entryHeader, entryStream, entryCallback) ->
									debug "Found #{entryHeader.name} in #{versionFile}"
									if entryHeader.name isnt 'package.xml' or entryHeader.type isnt 'file'
										entryStream.on 'end', -> do entryCallback
										# we have to consume the entire stream, before parsing continues
										do entryStream.resume
										return
									packageXmlFound = yes
									
									# set up xml parser
									packageXmlXmlStream = new xmlstream entryStream
									
									packageData = 
										time: versionFileStat.mtime
									
									listeners = 
										'startElement: package': (data) -> packageData.package = data.$.name
										'text: package > packageinformation > version':  (data) -> packageData.version = data.$text
										'text: package > packageinformation > license': (data) -> packageData.license = data.$text
										'text: package > packageinformation > isapplication': (data) -> packageData.isapplication = data.$text
										'text: package > packageinformation > packagename': (data) ->
											if not packageData.packagename? or not data.$?.language?
												packageData.packagename = data.$text
										'text: package > packageinformation > packagedescription': (data) ->
											if not packageData.packagedescription? or not data.$?.language?
												packageData.packagedescription = data.$text
										'text: package > authorinformation > author': (data) -> packageData.author = data.$text
										'text: package > authorinformation > authorurl': (data) -> packageData.authorurl = data.$text
										'text: package > requiredpackages > requiredpackage': (data) ->
											packageData.requiredpackages ?= []
											packageData.requiredpackages.push
												package: data.$text
												minversion: data.$?.minversion
										'text: package > excludedpackages > excludedpackage': (data) ->
											packageData.excludedpackages ?= []
											packageData.excludedpackages.push
												package: data.$text
												version: data.$?.version
										'text: package > optionalpackages > optionalpackage': (data) ->
											packageData.optionalpackages ?= []
											packageData.optionalpackages.push data.$text
										'startElement: package > instructions': (data) ->
											return if data.$?.type isnt 'update'
											return unless data.$?.fromversion?
											
											packageData.fromversions ?= []
											packageData.fromversions.push data.$.fromversion
										'end': ->
											debug "Finished parsing package.xml in #{versionFile}"
											
											do entryCallback
											unless packageData.package?
												callback "Package name missing in #{versionFile}"
												return
											if packageData.package isnt path.basename packageFolder
												callback "package name does not match folder in #{versionFile} (#{packageData.package} isnt #{path.basename packageFolder})"
												return
											unless packageData.version?
												callback "Version missing in #{versionFile}"
												return
											if (packageData.version.toLowerCase().replace /[ ]/g, '_') isnt path.basename versionFile, '.tar'
												debug "Boom"
												callback "version number does not match filename in #{versionFile} (#{packageData.version} isnt #{path.basename versionFile, '.tar'})"
												return
													
											callback null, packageData
									
									for event, listener of listeners
										packageXmlXmlStream.on event, listener
										
								archiveStream.pipe extract
									
							async.parallel tasks, (err, data) ->
								versionsCallback err, data
					, (err, data) ->
						# data = [ packageData, hash? ]
						
						data = data.filter (item) -> item?
						data.sort (a, b) ->
							a = a[0].version
							b = b[0].version
							
							if a is b
								0
							else if (createComparator "$v > #{b}")(a)
								1
							else
								-1
						if data.length is 0
							fileCallback "Could not find valid versions for #{packageFolder}"
						else
							fileCallback err, data
		, (err, data) ->
			if err?
				error "Error reading package list: #{err}"
				callback? false
				return
			
			data = data.filter (item) -> item?
			
			# overwrite packageList once everything succeeded
			packageList = data
			# update scan time and statistics
			lastUpdate = new Date
			updateTime = process.hrtime updateStart
			debug "Finished update"
			
			# and finally call the callback
			callback? true
			
askForCredentials = (req, res) ->
	res.type 'txt'
	res.setHeader 'WWW-Authenticate', 'Basic realm="Please provide proper username and password to access this package"'
	res.status(401).send 'Please provide proper username and password to access this package'
	
app.all '/', (req, res) ->
	callback = (username) ->
		host = config.basePath ? "#{req.protocol}://#{req.header 'host'}"
		
		# redirect when ?packageName=com.example.wcf.test[&packageVersion=1.0.0_Alpha_15] was requested
		if req.query?.packageName?
			if req.query.packageVersion?
				res.redirect 301, "#{host}/#{req.query.packageName}/#{req.query.packageVersion.replace /[ ]/g, '_'}"
			else
				res.redirect 301, "#{host}/#{req.query.packageName}"
			return
		
		unless req.accepts 'xml'
			res.sendStatus 406
			return
			
		res.type 'xml'
		
		# build the xml structure of the package list
		start = do process.hrtime
		writer = new xmlwriter true
		writer.startDocument '1.0', 'UTF-8'
		
		writer.startPI 'xml-stylesheet'
		writer.writeAttribute 'type', 'text/xsl'
		writer.writeAttribute 'href', "#{host}/style/main.xslt"
		do writer.endPI
		
		writer.startElement 'section'
		writer.writeAttribute 'name', 'packages'
		writer.writeAttribute 'xmlns', 'http://www.woltlab.com'
		writer.writeAttributeNS 'xmlns', 'xsi', 'http://www.w3.org/2001/XMLSchema-instance'
		writer.writeAttributeNS 'xsi', 'schemaLocation', 'http://www.woltlab.com https://www.woltlab.com/XSD/packageUpdateServer.xsd'
		for _package in packageList
			newestVersion = _package[-1..][0]
			writer.startElement 'package'
			writer.writeAttribute 'name', newestVersion[0].package
			writer.startElement 'packageinformation'
			
			writer.writeElement 'packagename', newestVersion[0].packagename ? ''
			writer.writeElement 'packagedescription', newestVersion[0].packagedescription ? ''
			writer.writeElement 'isapplication', String(newestVersion[0].isapplication ? 0)
			do writer.endElement
			
			# write <authorinformation>
			if newestVersion[0].author? or _package.authorurl?
				writer.startElement 'authorinformation'
				writer.writeElement 'author', newestVersion[0].author if newestVersion[0].author?
				writer.writeElement 'authorurl', newestVersion[0].authorurl if newestVersion[0].authorurl?
				do writer.endElement
			writer.startElement 'versions'
			
			for version in _package
				writer.startElement 'version'
				writer.writeAttribute 'name', version[0].version
				
				writer.writeAttribute 'accessible', if (isAccessible username, version[0].package, version[0].version) then "true" else "false"
				writer.writeAttribute 'critical', if (/pl/i.test version[0].version) then "true" else "false"
				writer.writeComment "sha256:#{version[1].toString 'hex'}" if version[1]?
				
				# write <fromversions>
				if version[0].fromversions?.length
					writer.startElement 'fromversions'
					for fromVersion in version[0].fromversions
						writer.writeElement 'fromversion', fromVersion
					do writer.endElement
				
				# write <optionalpackages>
				if version[0].optionalpackages?.length
					writer.startElement 'optionalpackages'
					for optionalpackage in version[0].optionalpackages
						writer.writeElement 'optionalpackage', optionalpackage
					do writer.endElement
				
				# write <requiredpackages>
				if version[0].requiredpackages?.length
					writer.startElement 'requiredpackages'
					for requiredPackage in version[0].requiredpackages
						writer.startElement 'requiredpackage'
						writer.writeAttribute 'minversion', requiredPackage.minversion if requiredPackage.minversion?
						writer.text requiredPackage.package
						do writer.endElement
					do writer.endElement
				
				# write <excludedpackages>
				if version[0].excludedpackages?.length
					writer.startElement 'excludedpackages'
					for excludedPackage in version[0].excludedpackages
						writer.startElement 'excludedpackage'
						writer.writeAttribute 'version', excludedPackage.version if excludedPackage.version?
						writer.text excludedPackage.package
						do writer.endElement
					do writer.endElement
				
				writer.writeElement 'timestamp', String(Math.floor (do version[0].time.getTime) / 1000)
				
				# e.g. #{host}/com.example.wcf.test/1.0.0_Alpha_15
				writer.writeElement 'file', "#{host}/#{version[0].package}/#{version[0].version.toLowerCase().replace /[ ]/g, '_'}"
				
				# try to extract license
				if version[0].license
					if result = /^(.*?)(?:\s<(https?:\/\/.*)>)?$/.exec version[0].license
						writer.startElement 'license'
						writer.writeAttribute 'url', result[2] if result[2]?
						writer.text result[1]
						do writer.endElement
					
				do writer.endElement
			
			do writer.endElement
			do writer.endElement
		diff = process.hrtime start
		unless config.deterministic
			writer.writeComment """
				xml generated in #{diff[0] + diff[1] / 1e9} seconds
				packages scanned in #{updateTime[0] + updateTime[1] / 1e9} seconds
				up since #{do process.uptime} seconds
				"""
		writer.writeComment "last update #{lastUpdate}"
		writer.writeComment "logged in as #{username}" if username
		writer.writeComment """
			This list was presented by Tim’s PackageServer #{serverVersion} <https://github.com/wbbaddons/Tims-PackageServer>
			Tim’s PackageServer is licensed under the terms of the GNU Affero General Public License v3 <https://gnu.org/licenses/agpl-3.0.html>.
			You can obtain a copy of the source code of this installation at #{host}/source/.
			"""
		do writer.endElement
		do writer.endDocument
		res.setHeader 'ETag', "#{if config.deterministic then '' else 'W/'}#{if username isnt '' then username + '-' else ''}#{lastUpdate.getTime()}"
		res.status(200).send writer.toString()
	
	checkAuth req, res, callback

# package download requested
app.all /^\/([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)\/([0-9]+\.[0-9]+\.[0-9]+(?:_(?:a|alpha|b|beta|d|dev|rc|pl)_[0-9]+)?)\/?(?:\?.*)?$/i, (req, res) ->
	callback = (username) ->
		fs.exists "#{config.packageFolder}/#{req.params[0]}/#{req.params[1].toLowerCase()}.tar", (packageExists) ->
			if packageExists
				if isAccessible username, req.params[0], req.params[1]
					debug "#{username} downloaded #{req.params[0]}/#{req.params[1].toLowerCase()}"
					logDownload req.params[0], req.params[1]
					res.download "#{config.packageFolder}/#{req.params[0]}/#{req.params[1].toLowerCase()}.tar", "#{req.params[0]}_v#{req.params[1]}.tar", (err) ->
						if err?
							res.sendStatus 404 unless res.headersSent
							
							do res.end
				else
					debug "#{username} tried to download #{req.params[0]}/#{req.params[1].toLowerCase()}"
					askForCredentials req, res
			else
				res.sendStatus 404
				
	checkAuth req, res, callback

# allow download without version number
app.all /^\/([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)\/?(?:\?.*)?$/i, (req, res) ->
	checkAuth req, res, (username) ->
		host = config.basePath ? "#{req.protocol}://#{req.header 'host'}"
		
		versionNumber = do ->
			# We have to check the whole package list here
			for _package in packageList
				# Check if current package is the wanted package
				if _package[-1..][0][0].package is req.params[0]
					# Check each version for accessibility, starting with the latest
					for version in _package by -1
						return version[0].version if isAccessible username, req.params[0], version[0].version
						
					return # No version for this package found
					
		unless versionNumber?
			res.sendStatus 404
			return
			
		res.redirect 303, "#{host}/#{req.params[0]}/#{versionNumber.toLowerCase().replace /[ ]/g, '_'}"

do ->
	sourceFiles = [
		'app.coffee'
		'views/main.handlebars'
	]
	
	app.get '/source', (req, res) ->
		host = config.basePath ? "#{req.protocol}://#{req.header 'host'}"
		res.format
			'text/html': ->
				res.type('html').send """
					<!doctype html>
					<html>
						<head>
							<title>#{config.pageTitle || 'Tim’s PackageServer'}</title>
						</head>
						<body>
							<p>The following source files are available for download:</p>
							<ul>
							#{sourceFiles.map((item) -> "<li><a href=\"#{host}/source/#{item}\">#{item}</a></li>").join "\n"}
							</ul>
						</body>
					</html>
					"""
			'text/plain': ->
				res.type('txt').send """
					The following source files are available for download:
					
					#{sourceFiles.map((item) -> "#{host}/source/#{item}").join "\n"}
					"""
			'default': -> res.sendStatus 406
		
	app.get new RegExp('/source/('+sourceFiles.join('|')+')'), (req, res) ->
		res.type('txt').sendFile "#{__dirname}/#{req.params[0]}", (err) ->
			if err
				if err.code is 'ECONNABORT' and res.statusCode is 304
					debug 'Request aborted, cached'
				else
					res.sendStatus 404 unless res.headersSent
					
				do res.end
			
app.get /\/style\/.*/, (req, res) ->
	checkAuth req, res, (username) ->
		res.type('text/xsl').render 'main',
			title: config.pageTitle || 'Tim’s PackageServer'
			serverVersion: serverVersion
			host: config.basePath ? "#{req.protocol}://#{req.header 'host'}"
			username: username
			lang:
				version: config.lang.version || 'Version'
				license: config.lang.license || 'License'
				requirements: config.lang.noRequirements || 'Requirements'
				date: config.lang.date || 'Date'
				by: config.lang.by || 'by'
				noRequirements: config.lang.noRequirements || 'No requirements found'
				missingLicenseInformation: config.lang.missingLicenseInformation || 'No license information'
				search: config.lang.search || 'Search…'
				
			
# throw 404 on any unknown route
app.all '*', (req, res) -> res.sendStatus 404

# Creates watchers for every relevant file in the package folder
do ->
	_readPackages = (require 'debounce') readPackages, 3e3
	watchr.watch
		path: config.packageFolder
		ignoreCustomPatterns: /\.txt$/
		listeners:
			watching: (err, watcherInstance, isWatching) ->
				if err
					error "watching the path #{watcherInstance.path} failed with error:", err
				else
					debug "watching the path #{watcherInstance.path} completed"
			change: (changeType, filePath, fileCurrentStat, filePreviousStat) ->
				debug "The package folder was changed (#{filePath}: #{changeType})"
				do _readPackages

# Once the package list was successfully scanned once bind to the port
readPackages -> app.listen config.port, config.ip
