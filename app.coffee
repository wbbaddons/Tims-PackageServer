###
Copyright (C) 2013 - 2015 Tim Düsterhus

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
	
process.chdir __dirname
serverVersion = (require './package.json').version
(require 'child_process').exec 'git describe --always', (err, stdout, stderr) -> serverVersion = stdout.trim() unless err?

authReader = require './authReader'
packageListReader = require './packageListReader'

async = require 'async'
basicAuth = require 'basic-auth'
bcrypt = require 'bcrypt'
bodyParser = require 'body-parser'
escapeRegExp = require 'escape-string-regexp'
express = require 'express'
expresshb  = require 'express-handlebars'
fs = require 'fs'
i18n = require 'i18n'
watchr = require 'watchr'
xmlwriter = require 'xml-writer'

debug = (require 'debug')('PackageServer:debug')
warn = (require 'debug')('PackageServer:warn')
warn.log = console.warn.bind console
error = (require 'debug')('PackageServer:error')
error.log = console.error.bind console

config = require('rc') 'PackageServer',
	port: 9001
	ip: '0.0.0.0'
	packageFolder: "#{__dirname}/packages/"
	enableStatistics: yes
	enableHash: yes
	deterministic: no
	ssl: no
	i18n:
		locales: [ 'en', 'de' ]
		directory: "#{__dirname}/locales"
		defaultLocale: 'en'
	
config.packageFolder += '/' unless /\/$/.test config.packageFolder

i18n.configure config.i18n

if config.enableManualUpdate?
	warn 'config.enableManualUpdate is obsolete and ignored in this version'
if config.basePath?
	warn "config.basePath is deprecated. Advice the reverse proxy to pass a proper 'Host' header"

process.title = "PackageServer #{config.packageFolder}"

# initialize express
app = do express

app.use i18n.init
app.use bodyParser.urlencoded extended: yes
app.engine 'handlebars', do expresshb
app.set 'view engine', 'handlebars'
app.set 'views', "#{__dirname}/views"
# app.enable 'view cache'

packageList = null
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

isAccessible = (username, testPackage, testVersion) ->
	return true if auth is null
	
	# check general packages
	if auth.packages?
		for _package, version of auth.packages
			return true if (version[0].test testPackage) and version[1] testVersion
	
	# check user
	if auth.users?[username]?.packages?
		for _package, version of auth.users[username].packages
			return true if (_package.test testPackage) and version testVersion
		# check user groups
		for group in auth.users[username].groups
			if auth.groups?[group]?
				for _package, version of auth.groups[group]
					return true if (version[0].test testPackage) and version[1] testVersion
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

# Updates package list.
readPackages = (callback) ->
	updateStart = do process.hrtime
	
	debug "Starting update"
	
	updateAuth = (callback) -> authReader "#{config.packageFolder}auth.json", callback
	updatePackageList = (callback) -> packageListReader config.packageFolder, config.enableHash, callback
	
	async.parallel [ updateAuth, updatePackageList ], (err, results) ->
		updateTime = process.hrtime updateStart
		lastUpdate = new Date
		
		if err?
			debug "Update failed:", err
			auth = packageList = undefined
			callback? err
		else
			debug "Finished update"
			[ auth, packageList ] = results
			callback? null
		
askForCredentials = (req, res) ->
	res.type 'txt'
	res.setHeader 'WWW-Authenticate', 'Basic realm="' + (req.__ 'Please provide proper username and password to access this package') + '"'
	res.status(401).send req.__ 'Please provide proper username and password to access this package'

app.use (req, res, next) ->
	res.set 'wcf-update-server-api', '2.0 2.1'
	res.set 'wcf-update-server-ssl', if config.ssl then "true" else "false"
	do next

app.all /^\/(?:list\/([a-z-]{2,})\.xml)?$/, (req, res) ->
	host = config.basePath ? "#{req.protocol}://#{req.header 'host'}"
	
	if req.query?.doAuth?
		checkAuth req, res, (username) ->
			if username
				res.redirect 303, host
			else
				askForCredentials req, res
		return

	callback = (username) ->
		# redirect when ?packageName=com.example.wcf.test[&packageVersion=1.0.0_Alpha_15] was requested
		if req.query?.packageName?
			if req.query.packageVersion?
				res.redirect 301, "#{host}/#{req.query.packageName}/#{req.query.packageVersion.replace /[ ]/g, '_'}"
			else
				res.redirect 301, "#{host}/#{req.query.packageName}"
			return
		
		unless packageList?
			res.sendStatus 503
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
			writer.writeElement 'packagename', newestVersion[0].packagename?[req.params[0] ? req.getLocale() ? '_'] ? newestVersion[0].packagename?['_'] ? ''
			writer.writeElement 'packagedescription', newestVersion[0].packagedescription?[req.params[0] ? req.getLocale() ? '_'] ? newestVersion[0].packagedescription?['_'] ? ''
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
			You can obtain a copy of the source code of this installation at <#{host}/source/>.
			"""
		do writer.endElement
		do writer.endDocument
		res.setHeader 'ETag', "#{if config.deterministic then '' else 'W/'}\"#{if username isnt '' then username + '-' else ''}#{lastUpdate.getTime()}\""
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
					
					if req.param('apiVersion') in [ '2.1' ]
						res.status(402).send req.__ "You may not access this package"
					else
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
		'authReader.coffee'
		'packageListReader.coffee'
		'streamHasher.coffee'
		'versionComparator.coffee'
		'versionComparator.jison'
		'views/main.handlebars'
		'views/source/html.handlebars'
		'views/source/txt.handlebars'
		'Dockerfile'
		'LICENSE'
	]
	
	app.get '/source', (req, res) ->
		host = config.basePath ? "#{req.protocol}://#{req.header 'host'}"
		res.format
			html: ->
				res.type('html').render 'source/html',
					title: config.pageTitle || 'Tim’s PackageServer'
					host: config.basePath ? "#{req.protocol}://#{req.header 'host'}"
					files: sourceFiles
			txt: ->
				res.type('txt').render 'source/txt',
					title: config.pageTitle || 'Tim’s PackageServer'
					host: config.basePath ? "#{req.protocol}://#{req.header 'host'}"
					files: sourceFiles
			default: -> res.sendStatus 406
		
	app.get new RegExp('/source/('+sourceFiles.map(escapeRegExp).join('|')+')'), (req, res) ->
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
