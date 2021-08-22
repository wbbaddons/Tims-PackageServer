// Copyright (C) 2013 - 2021 Tim Düsterhus
// Copyright (C) 2021 Maximilian Mader
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod error;
mod header;
pub mod helpers;
mod routes;

use crate::SETTINGS;
use actix_web::{middleware, App, HttpServer};
use helpers::{get_auth_info, is_accessible, redirect, RedirectType};
use routes::{
    about, assets, download, favicon, health, login, main_xslt, package_update_xml, source_code,
};

pub async fn run() -> crate::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(
                middleware::DefaultHeaders::new()
                    // 2.0: Standard package server
                    // 2.1: Supports `etag`, `last-modified`, `wcf-update-server-ssl`, status code 304 and the `/list/<lang>.xml` endpoint
                    // 3.1: Supports the '<compatibility>` element
                    .header("wcf-update-server-api", "2.0 2.1 3.1")
                    .header(
                        "wcf-update-server-ssl",
                        if SETTINGS.ssl { "true" } else { "false" },
                    ),
            )
            .service(health)
            .service(download())
            .service(main_xslt)
            .service(assets)
            .service(source_code())
            .service(favicon)
            .service(about)
            .service(login())
            .service(package_update_xml())
    })
    .bind((SETTINGS.ip, SETTINGS.port))?
    .run()
    .await
    .map_err(Into::into)
}
