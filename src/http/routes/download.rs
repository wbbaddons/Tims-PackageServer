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
        error::Error::{
            AccessDenied, PackageListUnavailable, PackageReadFailed, PaymentRequired,
            UnknownPackage, UnknownPackageVersion,
        },
        get_auth_info, is_accessible, redirect, RedirectType,
    },
    version::Version,
    AUTH_DATA, PACKAGE_LIST, SETTINGS,
};
use actix_web::{
    dev::HttpServiceFactory,
    http::header::{ContentDisposition, DispositionParam, DispositionType},
    middleware::{NormalizePath, TrailingSlash},
    web, HttpRequest, Responder,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{Mutex, RwLock},
};

static DOWNLOAD_COUNTERS: Lazy<RwLock<HashMap<String, Mutex<usize>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// Silence Clippy: We use this Mutex to lock the file write as well
#[allow(clippy::mutex_atomic)]
fn log_download(package_id: &str, version_str: &str) {
    fn write_counter_file(package_id: &str, version_str: &str, count: usize) {
        let path = PathBuf::from(&SETTINGS.package_dir)
            .join(package_id)
            .join(format!("{}.txt", version_str));

        let file = OpenOptions::new()
            .read(false)
            .write(true)
            .create(true)
            .append(false)
            .open(&path);

        match file {
            Ok(mut file) => {
                if let Err(err) = write!(file, "{}", count) {
                    log::error!(
                        "Failed to update download counter file \"{}\": {}",
                        path.display(),
                        err
                    );
                }
            }
            Err(err) => {
                log::error!(
                    "Failed to create or open download counter file \"{}\": {}",
                    path.display(),
                    err
                );
            }
        }
    }

    log::trace!("Logging download for {}/{}", package_id, version_str);

    let key = format!("{}_{}", package_id, version_str);

    let map = DOWNLOAD_COUNTERS.read().unwrap();

    if let Some(counter) = map.get(&key) {
        let mut counter = counter.lock().unwrap();
        *counter += 1;

        return write_counter_file(package_id, version_str, *counter);
    }

    // The key did not exists, drop the read lock ...
    std::mem::drop(map);
    // ... and accquire a write lock instead
    let mut map = DOWNLOAD_COUNTERS.write().unwrap();

    // Two requests could try to create this key “simultaneously”,
    // thus we insert a zero and increment afterwards by locking the mutex itself.
    let counter = map.entry(key).or_insert_with(|| {
        // Try to read the current count
        let path = PathBuf::from(&SETTINGS.package_dir)
            .join(package_id)
            .join(format!("{}.txt", version_str));

        let count = std::fs::read(path)
            .ok()
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Mutex::new(count)
    });

    let counter = counter.get_mut().unwrap();
    *counter += 1;

    write_counter_file(package_id, version_str, *counter);
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    api_version: Option<String>,
    package_name: Option<String>,
    package_version: Option<String>,
}

pub fn download() -> impl HttpServiceFactory {
    web::scope("/{package_id:[a-zA-Z0-9_-]+\\.[a-zA-Z0-9_-]+(?:\\.[a-zA-Z0-9_-]+)+}")
        .wrap(NormalizePath::new(TrailingSlash::Always))
        .service(
            web::resource(
                "/{version:[0-9]+\\.[0-9]+\\.[0-9]+(?:_(?:a|alpha|b|beta|d|dev|rc|pl)_[0-9]+)?}/",
            )
            .route(web::get().to(get_download_package))
            .route(web::post().to(post_download_package)),
        )
        .service(
            web::resource("/latest/")
                .route(web::get().to(download_latest))
                .route(web::post().to(download_latest)),
        )
        .service(
            web::resource("/")
                .route(web::get().to(download_latest))
                .route(web::post().to(download_latest)),
        )
}

async fn download_latest(
    req: HttpRequest,
    host: crate::http::header::Host,
    auth: Option<BasicAuth>,
    package_id: web::Path<String>,
) -> impl Responder {
    let auth_data = AUTH_DATA.load_full();
    let auth_info = get_auth_info(&auth_data, auth);

    if let Some(package_list) = PACKAGE_LIST.load_full() {
        'outer: for package in &package_list.packages {
            for version in package.iter().rev() {
                if version.data.name != package_id.as_str() {
                    continue 'outer;
                }

                let version = &version.data.package_information.version;

                if is_accessible(&package_id, version, &auth_info, &auth_data) {
                    return Ok(redirect(RedirectType::Other(format!(
                        "{}/{}/{}/",
                        *host,
                        package_id,
                        version.format_url()
                    ))));
                }
            }

            // The package exists but the user is not authorized to download it
            return Err(AccessDenied(req));
        }

        // No version found
        Err(UnknownPackage(req, package_id.to_string()))
    } else {
        Err(PackageListUnavailable(req))
    }
}

async fn get_download_package(
    req: HttpRequest,
    auth: Option<BasicAuth>,
    path: web::Path<(String, String)>,
    web::Query(query): web::Query<DownloadRequest>,
) -> impl Responder {
    download_package(req, auth, path, query)
}

async fn post_download_package(
    req: HttpRequest,
    auth: Option<BasicAuth>,
    path: web::Path<(String, String)>,
    web::Query(query): web::Query<DownloadRequest>,
    web::Form(params): web::Form<DownloadRequest>,
) -> impl Responder {
    let params = DownloadRequest {
        api_version: params.api_version.or(query.api_version),
        package_name: params.package_name.or(query.package_name),
        package_version: params.package_version.or(query.package_version),
    };

    download_package(req, auth, path, params)
}

fn download_package(
    req: HttpRequest,
    auth: Option<BasicAuth>,
    path: web::Path<(String, String)>,
    params: DownloadRequest,
) -> impl Responder {
    let (package_id, version_str) = path.into_inner();
    let auth_data = AUTH_DATA.load_full();
    let auth_info = get_auth_info(&auth_data, auth);
    // The path makes sure that the version is valid
    let version = Version::try_from(version_str.replace('_', " ").as_str()).unwrap();

    let filename = format!("{}.tar", version_str);
    let download_name = format!("{}_v{}.tar", &package_id, version_str);

    let file_path = SETTINGS.package_dir.join(&package_id).join(&filename);

    if is_accessible(&package_id, &version, &auth_info, &auth_data) {
        let file = match actix_files::NamedFile::open(file_path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(UnknownPackageVersion(req, package_id, version));
            }
            Err(err) => {
                log::error!(
                    "Failed to read authorized package {} v{}:{:?}",
                    package_id,
                    version,
                    err
                );

                return Err(PackageReadFailed(req, download_name));
            }
        };

        let cd = ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename(download_name)],
        };

        if SETTINGS.enable_statistics {
            log_download(&package_id, &version_str);
        }

        return Ok(file
            .use_etag(true)
            .use_last_modified(true)
            .set_content_disposition(cd)
            .into_response(&req));
    } else if file_path.is_file() {
        let who = auth_info
            .username
            .or_else(|| req.peer_addr().map(|addr| addr.to_string()))
            .unwrap_or_else(|| "An anonymous user".to_owned());

        log::debug!("{} tried to download {}/{}", who, package_id, version_str);

        match params.api_version {
            Some(api_version) if api_version == "2.1" => {
                return Err(PaymentRequired(req, package_id, version));
            }
            _ => return Err(AccessDenied(req)),
        }
    }

    Err(UnknownPackage(req, package_id))
}
