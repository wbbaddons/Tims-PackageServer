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
    http::{error::Error::AccessDenied, get_auth_info, redirect, RedirectType},
    AUTH_DATA,
};
use actix_web::{
    dev::HttpServiceFactory,
    middleware::{NormalizePath, TrailingSlash},
    web, HttpRequest, Responder,
};
use actix_web_httpauth::extractors::basic::BasicAuth;

pub fn login() -> impl HttpServiceFactory {
    web::scope("/login")
        .wrap(NormalizePath::new(TrailingSlash::Always))
        .service(
            web::resource("/")
                .route(web::get().to(perform_login))
                .route(web::post().to(perform_login)),
        )
}

async fn perform_login(req: HttpRequest, auth: Option<BasicAuth>) -> impl Responder {
    let auth_data = AUTH_DATA.load_full();
    let auth_info = get_auth_info(&auth_data, auth);

    auth_info
        .username
        .map(|_| redirect(RedirectType::Other("/".to_owned())))
        .ok_or(AccessDenied(req))
}
