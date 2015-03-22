# Copyright (C) 2013 - 2015 Tim Düsterhus
# 
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
# 
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# 
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

FROM node:onbuild
MAINTAINER Tim Düsterhus

RUN	groupadd -r node \
	&& useradd -r -g node node \
	&& chown -R node:node /usr/src/app

USER node

EXPOSE 9001
VOLUME [ "/usr/src/app/packages" ]
