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

createComparator = require './versionComparator'
streamHasher = require './streamHasher'

async = require 'async'
fs = require 'fs'
path = require 'path'
tarstream = require 'tar-stream'
xmlstream = require 'xml-stream'

debug = (require 'debug')('PackageServer:packageListReader')
warn = (require 'debug')('PackageServer:packageListReader:warn')
warn.log = console.warn.bind console
error = (require 'debug')('PackageServer:packageListReader:error')
error.log = console.error.bind console

module.exports = (folder, enableHash, callback) ->
	fs.readdir folder, (err, files) ->
		if err?
			callback err
			return
		
		# ignore dotfiles, auth.json and invalid package identifiers
		files = files.filter (file) ->
			if file[0] is '.'
				debug "Skipping dotfile #{folder}#{file}"
				false
			else if file in [ 'auth.json', 'auth.json.example' ]
				false
			else unless /^([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)$/i.test file
				debug "Skipping #{folder}#{file}, as it does not match a valid package identifier"
				false
			else
				true
		
		# loop over each file in the package folder
		async.map files, (file, fileCallback) ->
			packageFolder = folder + file
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
							
							if enableHash
								tasks.push (callback) -> streamHasher archiveStream, callback
							
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
											packageData.packagename ?= { }
											packageData.packagename[data.$?.language ? '_'] = data.$text
										'text: package > packageinformation > packagedescription': (data) ->
											packageData.packagedescription ?= { }
											packageData.packagedescription[data.$?.language ? '_'] = data.$text
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
												callback "version number does not match filename in #{versionFile} (#{packageData.version} isnt #{path.basename versionFile, '.tar'})"
												return
											
											callback null, packageData
									
									for event, listener of listeners
										packageXmlXmlStream.on event, listener
										
								archiveStream.pipe extract
									
							async.parallel tasks, (err, data) ->
								versionsCallback err, data
					, (err, data) ->
						# data = [ [ packageData, hash? ], … ]
						if err?
							fileCallback err
						else
							data = data.filter (item) -> item?
							data.sort (a, b) ->
								a = a[0].version
								b = b[0].version
								
								if a is b
									0
								else if (createComparator "#{a} > #{b}")()
									1
								else
									-1
							
							if data.length is 0
								warn "Could not find valid versions for #{packageFolder}"
								# this will be filtered out afterwards
								fileCallback null, undefined
							else
								fileCallback err, data
		, (err, data) ->
			if err?
				error "Error reading package list: #{err}"
				packageList = null
				callback err
				return
			
			data = data.filter (item) -> item?
			
			# and finally call the callback
			callback null, data
