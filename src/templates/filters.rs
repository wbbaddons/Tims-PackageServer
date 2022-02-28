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
    auth::AuthData,
    package::{
        list_reader::PackageInfo,
        xml,
        xml::{PackageDescription, PackageName},
    },
    templates::AuthInfo,
};
use std::time::SystemTime;
use unic_langid::LanguageIdentifier;

pub fn is_accessible(
    package: &PackageInfo,
    auth_info: &AuthInfo,
    auth_data: &AuthData,
) -> askama::Result<&'static str> {
    let version = &package.data.package_information.version;
    let package_id = &package.data.name;

    if crate::http::helpers::is_accessible(package_id, version, auth_info, auth_data) {
        Ok("true")
    } else {
        Ok("false")
    }
}

pub fn timestamp(t: SystemTime) -> askama::Result<u64> {
    let duration = t
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| askama::Error::from(std::fmt::Error))?;

    Ok(duration.as_secs())
}

pub fn rfc3339(t: &SystemTime) -> askama::Result<String> {
    Ok(humantime::format_rfc3339(*t).to_string())
}

fn get_for_language<'a, T: xml::HasLanguage>(
    items: &'a [T],
    lang: &Option<LanguageIdentifier>,
) -> Option<&'a T> {
    None.or_else(|| items.iter().find(|item| *item.language() == *lang))
        .or_else(|| items.iter().find(|item| item.language().is_none()))
        .or_else(|| items.first())
}

pub fn package_name(
    names: &[PackageName],
    lang: &Option<LanguageIdentifier>,
) -> askama::Result<String> {
    let result = get_for_language(names, lang)
        .map_or_else(|| "".to_owned(), |package_name| package_name.name.clone());

    Ok(result)
}

pub fn package_description(
    descriptions: &[PackageDescription],
    lang: &Option<LanguageIdentifier>,
) -> askama::Result<String> {
    let result = get_for_language(descriptions, lang).map_or_else(
        || "".to_owned(),
        |description| description.description.clone(),
    );

    Ok(result)
}
