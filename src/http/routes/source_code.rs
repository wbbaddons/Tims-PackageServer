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
        error::Error::{FileNotFound, NotAcceptable},
        header::{not_modified, Host, Language},
    },
    templates::{SourceCodeHtmlTemplate, SourceCodeTextTemplate},
};
use actix_web::{
    dev::HttpServiceFactory,
    get,
    http::header::{
        Accept, CacheControl, CacheDirective, ETag, EntityTag, Header, CACHE_CONTROL, ETAG, VARY,
    },
    web, Either, HttpRequest, HttpResponse, Responder,
};

pub fn source_code() -> impl HttpServiceFactory {
    web::scope("/source").service(index).service(get_file)
}

enum OutputType {
    Html,
    Plain,
}

#[get("/")]
async fn index(req: HttpRequest, language: Language, host: Host) -> impl Responder {
    use askama_actix::TemplateIntoResponse;

    let accept = Accept::parse(&req);
    let output_type = match accept.as_ref().ok() {
        Some(accept) if !accept.mime_precedence().is_empty() => accept
            .mime_precedence()
            .iter()
            .find_map(|mime| match (mime.type_(), mime.subtype()) {
                (mime::TEXT, mime::HTML) => Some(OutputType::Html),
                (mime::TEXT, mime::PLAIN) => Some(OutputType::Plain),
                (mime::STAR, _) => Some(OutputType::Html),
                _ => None,
            }),
        Some(_) | None => Some(OutputType::Plain),
    };

    match output_type {
        Some(OutputType::Html) => {
            let etag = ETag(EntityTag::new(
                false,
                format!(
                    "html-{}-{}",
                    language.to_string(),
                    crate::SOURCE_FILES_COMBINED_HASH
                ),
            ));

            if not_modified(&req, Some(&etag), None) {
                return Ok(Either::B(
                    HttpResponse::NotModified()
                        .set_header(CACHE_CONTROL, CacheControl(vec![CacheDirective::Public]))
                        .set_header(ETAG, etag)
                        .set_header(VARY, "accept, accept-language")
                        .body(actix_web::body::Body::None),
                ));
            }

            Ok(Either::A(
                SourceCodeHtmlTemplate {
                    host: host.clone(),
                    server_version: crate::built_info::version(),
                    title: crate::SETTINGS.page_title.as_ref(),
                    lang: language.to_string(),
                }
                .into_response()
                .with_header(CACHE_CONTROL, CacheControl(vec![CacheDirective::Public]))
                .with_header(ETAG, etag)
                .with_header(VARY, "accept, accept-language"),
            ))
        }
        Some(OutputType::Plain) => {
            let etag = ETag(EntityTag::new(
                false,
                format!(
                    "txt-{}-{}",
                    language.to_string(),
                    crate::SOURCE_FILES_COMBINED_HASH
                ),
            ));

            if not_modified(&req, Some(&etag), None) {
                return Ok(Either::B(
                    HttpResponse::NotModified()
                        .set_header(CACHE_CONTROL, CacheControl(vec![CacheDirective::Public]))
                        .set_header(ETAG, etag)
                        .set_header(VARY, "accept, accept-language")
                        .body(actix_web::body::Body::None),
                ));
            }

            Ok(Either::A(
                SourceCodeTextTemplate {
                    host: host.clone(),
                    server_version: crate::built_info::version(),
                    title: crate::SETTINGS.page_title.as_ref(),
                    lang: language.to_string(),
                }
                .into_response()
                .with_header(CACHE_CONTROL, CacheControl(vec![CacheDirective::Public]))
                .with_header(ETAG, etag)
                .with_header(VARY, "accept, accept-language"),
            ))
        }
        None => {
            // This unwrap is safe because the header must have
            // been parsed successfully to land in this case
            Err(NotAcceptable(req, accept.unwrap()))
        }
    }
}

#[get("/{filename:.*}")]
async fn get_file(req: HttpRequest, web::Path(filename): web::Path<String>) -> impl Responder {
    if let Some(source_file) = crate::SOURCE_FILES.get(&filename) {
        let content_type = mime_guess::from_path(filename)
            .first()
            .map(|mime| {
                // Let browsers try to display the file directly
                if mime.type_() == mime::TEXT {
                    return mime::TEXT_PLAIN_UTF_8;
                }

                mime
            })
            .unwrap_or(mime::TEXT_PLAIN_UTF_8)
            .to_string();

        return Ok(HttpResponse::Ok()
            .set(ETag::from(source_file))
            .set(CacheControl(vec![CacheDirective::Public]))
            .content_type(content_type)
            .body(source_file.contents));
    }

    Err(FileNotFound(req, filename))
}
