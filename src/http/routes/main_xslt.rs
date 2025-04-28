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

use crate::{
    http::{
        get_auth_info,
        header::{Host, Language},
        SETTINGS,
    },
    templates::{MainTemplate, Template},
    AUTH_DATA, LICENSE_INFO,
};
use actix_web::{get, http::header::VARY, HttpResponse, Responder};
use actix_web_httpauth::extractors::basic::BasicAuth;

#[get("/style/main.xslt")]
pub async fn main_xslt(
    auth: Option<BasicAuth>,
    language: Language,
    host: Host,
) -> std::io::Result<impl Responder> {
    let auth_data = AUTH_DATA.load_full();
    let auth_info = get_auth_info(&auth_data, auth);

    Ok(HttpResponse::Ok()
        .insert_header((VARY, "accept-language"))
        .body(
            MainTemplate {
                host: host.clone(),
                server_version: crate::built_info::version(),
                title: SETTINGS.page_title.as_ref(),
                license_info: LICENSE_INFO,
                lang: language.to_string(),
                auth_data,
                auth_info,
            }
            .render()
            .map_err(|err| err.into_io_error())?,
        ))
}
