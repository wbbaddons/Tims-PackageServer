# Copyright (C) 2013 - 2021 Tim Düsterhus
# Copyright (C) 2021 Maximilian Mader
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
# along with this program.  If not, see <http:#www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

FROM	node:24 AS npm

COPY	package.json package-lock.json /usr/src/PackageServer/

RUN	cd /usr/src/PackageServer \
&&	npm ci

FROM	rust:1 AS builder

COPY	. /usr/src/PackageServer/
COPY	--from=npm /usr/src/PackageServer/node_modules/ /usr/src/PackageServer/node_modules/

RUN	cd /usr/src/PackageServer/ \
&&	find . -not -path './.git/*' \
&&	cargo build --locked --release

FROM	debian:bookworm-slim

RUN	set -ex \
&&	groupadd -r packageserver \
&&	useradd -r -g packageserver packageserver \
&&	mkdir /var/lib/PackageServer/ \
&&	mkdir /var/lib/PackageServer/packages/ \
&&	chown packageserver:packageserver /var/lib/PackageServer/ /var/lib/PackageServer/packages/

VOLUME	[ "/var/lib/PackageServer/packages" ]

COPY	--from=builder /usr/src/PackageServer/target/release/tims-package-server /usr/local/bin/tims-package-server

WORKDIR	/var/lib/PackageServer/

USER	packageserver
EXPOSE	9001

CMD	[ "tims-package-server" ]
