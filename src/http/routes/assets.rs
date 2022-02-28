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

use crate::http::{
    error::{Error, Error::FileNotFound},
    header::not_modified,
};
use actix_web::{
    get,
    http::header::{CacheControl, CacheDirective, ETag},
    web, HttpRequest, HttpResponse, Responder,
};

#[get("/favicon.ico")]
pub async fn favicon(req: HttpRequest) -> impl Responder {
    serve_asset(req, "favicon.ico")
}

#[get("/static/{filename:.+}")]
pub async fn assets(req: HttpRequest, filename: web::Path<String>) -> impl Responder {
    serve_asset(req, &filename)
}

fn serve_asset(req: HttpRequest, filename: &str) -> Result<impl Responder, Error> {
    let file = format!("assets/static/{}", filename);

    crate::SOURCE_FILES
        .get(file.as_str())
        .map(|source_file| {
            let etag = ETag::from(source_file);

            if not_modified(&req, Some(&etag), None) {
                return HttpResponse::NotModified()
                    .insert_header(etag)
                    .insert_header(CacheControl(vec![CacheDirective::Public]))
                    .body(()); // None
            }

            let content_type = if filename.ends_with(".js.map") {
                "application/json".to_owned()
            } else {
                mime_guess::from_path(filename)
                    .first_or_octet_stream()
                    .to_string()
            };

            HttpResponse::Ok()
                .insert_header(etag)
                .insert_header(CacheControl(vec![CacheDirective::Public]))
                .content_type(content_type)
                .body(source_file.contents)
        })
        .ok_or(FileNotFound(req, file))
}
