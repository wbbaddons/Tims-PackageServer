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

mod filters;

use crate::{
    auth::AuthData, fluent, http::helpers::AuthInfo, package::list_reader::PackageList, LicenseInfo,
};
use askama_actix::Template;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[derive(Template)]
#[template(path = "main.xslt", escape = "xml")]
pub struct MainTemplate {
    pub host: String,
    pub server_version: String,
    pub title: Option<&'static String>,
    pub license_info: LicenseInfo,
    pub lang: String,
    pub auth_data: Arc<AuthData>,
    pub auth_info: AuthInfo,
}

impl MainTemplate {
    fn asset(&self, name: &str) -> String {
        format!("{}/{}", self.host, name)
    }

    fn sri(&self, name: &str) -> &'static str {
        crate::SOURCE_FILES
            .get(format!("assets/{}", name).as_str())
            .map_or(
                // https://w3c.github.io/webappsec-subresource-integrity/#the-integrity-attribute
                // > The value of the attribute MUST be either the empty string, or at least one valid metadata
                "",
                |source_file| source_file.sha384_digest,
            )
    }
}

#[derive(Template)]
#[template(path = "packageUpdateServer.xml")]
pub struct PackageUpdateXmlTemplate {
    pub host: String,
    pub server_version: String,
    pub package_list: Arc<PackageList>,
    pub user_lang: String,
    pub xml_lang: Option<LanguageIdentifier>,
    pub auth_data: Arc<AuthData>,
    pub auth_info: AuthInfo,

    pub deterministic: bool,
    pub uptime: std::time::Duration,
    pub start_time: std::time::Instant,
}

#[derive(Template)]
#[template(path = "source/source.html")]
pub struct SourceCodeHtmlTemplate {
    pub host: String,
    pub server_version: String,
    pub title: Option<&'static String>,
    pub lang: String,
}

#[derive(Template)]
#[template(path = "source/source.txt")]
pub struct SourceCodeTextTemplate {
    pub host: String,
    pub server_version: String,
    pub title: Option<&'static String>,
    pub lang: String,
}
