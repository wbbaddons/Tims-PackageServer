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
        error::Error::{NotAcceptable, PackageListUnavailable},
        get_auth_info,
        header::{negotiate_language, not_modified, Host, Language},
        redirect, RedirectType, SETTINGS,
    },
    templates::PackageUpdateXmlTemplate,
    AUTH_DATA, PACKAGE_LIST, UPTIME,
};
use actix_web::{
    dev::HttpServiceFactory,
    http::header::{
        Accept, ETag, EntityTag, Header, LastModified, CONTENT_TYPE, ETAG, LAST_MODIFIED, VARY,
    },
    web, Either, HttpRequest, HttpResponse, Responder,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as BASE64, Engine as _};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageUpdateXmlRequest {
    package_name: Option<String>,
    package_version: Option<String>,
}

pub fn package_update_xml() -> impl HttpServiceFactory {
    web::scope("")
        .service(
            web::resource("/")
                .route(web::get().to(get_xml))
                .route(web::post().to(post_xml)),
        )
        .service(
            web::resource("/list/{lang:[a-zA-Z0-9_-]+}.xml")
                .route(web::get().to(get_xml))
                .route(web::post().to(post_xml)),
        )
}

async fn get_xml(
    req: HttpRequest,
    auth: Option<BasicAuth>,
    user_lang: Language,
    host: Host,
    web::Query(query): web::Query<PackageUpdateXmlRequest>,
) -> impl Responder {
    response(req, auth, user_lang, host, query)
}

async fn post_xml(
    req: HttpRequest,
    auth: Option<BasicAuth>,
    user_lang: Language,
    host: Host,
    web::Query(query): web::Query<PackageUpdateXmlRequest>,
    web::Form(params): web::Form<PackageUpdateXmlRequest>,
) -> impl Responder {
    let params = PackageUpdateXmlRequest {
        package_name: params.package_name.or(query.package_name),
        package_version: params.package_version.or(query.package_version),
    };

    response(req, auth, user_lang, host, params)
}

fn response(
    req: HttpRequest,
    auth: Option<BasicAuth>,
    mut user_lang: Language,
    host: Host,
    params: PackageUpdateXmlRequest,
) -> impl Responder {
    let accept = Accept::parse(&req);
    let is_acceptable = match accept.as_ref().ok() {
        Some(accept) if !accept.ranked().is_empty() => accept.ranked().iter().any(|mime| {
            matches!(
                (mime.type_(), mime.subtype()),
                (mime::TEXT, mime::XML) | (mime::STAR, _)
            )
        }),
        Some(_) | None => true,
    };

    if !is_acceptable {
        return Err(NotAcceptable(req, accept.unwrap()));
    }

    let xml_lang = req
        .match_info()
        .get("lang")
        .and_then(|lang| lang.parse().ok());

    // Try to override the fluent language
    if let Some(ref lang_id) = xml_lang {
        user_lang = Language(negotiate_language(&[lang_id]));
    }

    let user_lang_string = user_lang.to_string();
    let auth_data = AUTH_DATA.load_full();
    let auth_info = get_auth_info(&auth_data, auth);
    let package_list = PACKAGE_LIST.load_full();

    match package_list {
        Some(package_list) => {
            if req.path() == "/" {
                if let Some(name) = params.package_name {
                    let url = params
                        .package_version
                        .map(|version| format!("{}/{}/{}/", *host, name, version))
                        .unwrap_or_else(|| format!("{}/{}/", *host, name));

                    return Ok(Either::Right(redirect(RedirectType::Permanent(url))));
                }
            }

            let timestamp = package_list
                .updated_at
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let etag_content = match (&auth_info.username, &xml_lang) {
                (Some(username), Some(xml_lang)) => {
                    format!(
                        "{},{},{},{}",
                        timestamp, user_lang_string, username, xml_lang
                    )
                }
                (Some(username), None) => {
                    format!("{},{},{},", timestamp, user_lang_string, username)
                }
                (None, Some(xml_lang)) => {
                    format!("{},{},,{}", timestamp, user_lang_string, xml_lang)
                }
                (None, None) => {
                    format!("{},{}", timestamp, user_lang_string)
                }
            };

            let etag = ETag(EntityTag::new(
                !SETTINGS.deterministic,
                BASE64.encode(etag_content),
            ));
            let last_modified = LastModified(package_list.updated_at.into());

            if not_modified(&req, Some(&etag), Some(*last_modified)) {
                return Ok(Either::Right(
                    HttpResponse::NotModified()
                        .insert_header((ETAG, etag))
                        .insert_header((LAST_MODIFIED, last_modified))
                        .insert_header((VARY, "accept, accept-language"))
                        .body(()),
                ));
            }

            Ok(Either::Left(
                PackageUpdateXmlTemplate {
                    host: host.clone(),
                    server_version: crate::built_info::version(),
                    package_list,
                    user_lang: user_lang_string,
                    xml_lang,
                    auth_data,
                    auth_info,

                    uptime: UPTIME.get().unwrap().elapsed(),
                    deterministic: SETTINGS.deterministic,
                    start_time: std::time::Instant::now(),
                }
                .customize()
                .insert_header((ETAG, etag))
                .insert_header((LAST_MODIFIED, last_modified))
                .insert_header((CONTENT_TYPE, "text/xml; charset=utf-8"))
                .insert_header((VARY, "accept, accept-language")),
            ))
        }
        None => Err(PackageListUnavailable(req)),
    }
}
