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

fs = require 'fs'
oboe = require 'oboe'
once = require 'once'

debug = (require 'debug')('PackageServer:authReader')
warn = (require 'debug')('PackageServer:authReader:warn')
warn.log = console.warn.bind console
error = (require 'debug')('PackageServer:authReader:error')
error.log = console.error.bind console

module.exports = (filename, callback) ->
	fs.exists filename, (exists) ->
		# no auth.json was found
		unless exists
			warn "auth.json does not exist. Assuming all packages are free"
			callback null, null
			return
			
		stream = fs.createReadStream filename
		
		debug 'Starting auth.json parsing'
		data = oboe stream
		data.fail once (err) ->
			callback "Error parsing auth.json: #{err.thrown}"
		data.node 'packages.*', (item) ->
			debug "Converting #{item} into comparator"
			createComparator item
		data.done (json) ->
			debug "Finished parsing auth"
			callback null, json
