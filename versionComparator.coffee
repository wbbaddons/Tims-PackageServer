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

debug = (require 'debug')('PackageServer:versionComparator')

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

module.exports = (comparison) ->
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
	
	comparison = comparison.replace /\$v(==|<=|>=|<|>)([0-9]+\.[0-9]+\.[0-9]+.-?[0-9]+.[0-9]+)/g, (comparison, operator, v2) -> """(comparatorHelper($v, "#{v2}") #{operator} 0)"""
	comparison = comparison.replace /\$v~(\/(?:[^/\\]|\\.)+\/i?)/g, (comparison, regex) -> """(#{regex}.test($v))"""
	comparison = comparison.replace /\$v!~(\/(?:[^/\\]|\\.)+\/i?)/g, (comparison, regex) -> """(!#{regex}.test($v))"""
	
	debug "Result: #{comparison}"
	comparator = new Function '$v', 'comparatorHelper', 'return ' + comparison
	
	($v) ->	comparator $v, comparatorHelper
