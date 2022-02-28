//! Copyright (C) 2013 - 2021 Tim Düsterhus
//! Copyright (C) 2021 Maximilian Mader
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU Affero General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//! GNU Affero General Public License for more details.
//!
//! You should have received a copy of the GNU Affero General Public License
//! along with this program.  If not, see <http://www.gnu.org/licenses/>.
//!
//! SPDX-License-Identifier: AGPL-3.0-or-later

include!(concat!(env!("OUT_DIR"), "/licence_info.rs"));

use crate::{
    auth::AuthData,
    package::{list_reader::PackageList, watcher::PackageWatcher},
};
use arc_swap::{ArcSwap, ArcSwapOption};
use config::Config;
use futures_util::TryFutureExt;
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv6Addr},
    path::PathBuf,
    sync::Arc,
};

mod auth;
mod built_info;
mod fluent;
mod http;
mod nom;
mod package;
mod source_files;
mod templates;
mod version;

pub use source_files::*;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub port: u16,
    pub ip: IpAddr,
    pub package_dir: PathBuf,
    pub enable_statistics: bool,
    pub deterministic: bool,
    pub ssl: bool,

    pub page_title: Option<String>,
    pub host: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port: 9001,
            ip: Ipv6Addr::UNSPECIFIED.into(),
            package_dir: std::path::PathBuf::from("packages")
                .canonicalize()
                .expect("A valid package directory"),
            enable_statistics: true,
            deterministic: true,
            ssl: false,

            page_title: None,
            host: None,
        }
    }
}

pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
    let mut settings: Settings = Config::builder()
        .add_source(Config::try_from(&Settings::default()).unwrap())
        .add_source(config::File::with_name("PackageServer_config").required(false))
        .add_source(config::Environment::with_prefix("PackageServer"))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();

    settings.package_dir = settings
        .package_dir
        .canonicalize()
        .expect("A valid package directory");
    settings
});

pub static PACKAGE_LIST: Lazy<ArcSwapOption<PackageList>> = Lazy::new(ArcSwapOption::empty);
pub static AUTH_DATA: Lazy<ArcSwap<AuthData>> =
    Lazy::new(|| ArcSwap::from_pointee(AuthData::default()));
pub static UPTIME: OnceCell<std::time::Instant> = OnceCell::new();
pub static WATCHER: OnceCell<PackageWatcher> = OnceCell::new();

async fn init_auth_data() -> crate::Result<()> {
    let path = SETTINGS.package_dir.join("auth.json");
    let auth_data = if path.exists() {
        match auth::read_auth_json(path).map_err(Into::into) {
            Ok(auth_data) => auth_data,
            Err(err) => {
                log::error!("Failed to read `auth.json`: {}", err);
                return Err(err);
            }
        }
    } else {
        AuthData::default()
    };

    AUTH_DATA.store(Arc::new(auth_data));

    Ok(())
}

async fn init_package_list() -> crate::Result<()> {
    let package_list = match package::list_reader::scan_packages() {
        Ok(package_list) => package_list,
        Err(err) => {
            log::error!("Failed to read package directory: {}", err);
            return Err(err);
        }
    };

    PACKAGE_LIST.store(Some(Arc::new(package_list)));

    Ok(())
}

#[actix_web::main]
async fn main() -> crate::Result<()> {
    if unsafe { ::libc::getuid() } == 0 {
        panic!("Cowardly refusing to keep the process alive as root.");
    }

    UPTIME
        .set(std::time::Instant::now())
        .expect("setting UPTIME to succeed");

    let env = env_logger::Env::default()
        .default_filter_or("tims_package_server=info")
        .default_write_style_or("auto");

    env_logger::init_from_env(env);

    futures::try_join!(
        http::run(),
        init_auth_data(),
        init_package_list().and_then(|_| async {
            let watcher = PackageWatcher::new(&SETTINGS.package_dir);

            match watcher {
                Ok(watcher) => {
                    WATCHER.set(watcher).expect("the OnceCell to be empty");
                }
                Err(err) => {
                    log::error!("Failed to start FS watcher: {}", err);
                }
            }

            Ok(())
        })
    )?;

    Ok(())
}
