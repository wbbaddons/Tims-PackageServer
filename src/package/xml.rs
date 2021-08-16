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

use crate::version::Version;
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::{convert::TryFrom, fmt::Display};
use unic_langid::LanguageIdentifier;

static LICENSE_REGEX: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r#"^(.*?)(?:\s<(https?://.*)>)?$"#)
        .case_insensitive(true)
        .build()
        .unwrap()
});

pub trait HasLanguage {
    fn language(&self) -> &Option<LanguageIdentifier>;
}

#[derive(Debug, Default)]
pub struct PackageName {
    pub name: String,
    pub language: Option<LanguageIdentifier>,
}

impl HasLanguage for PackageName {
    fn language(&self) -> &Option<LanguageIdentifier> {
        &self.language
    }
}

#[derive(Debug, Default)]
pub struct PackageDescription {
    pub description: String,
    pub language: Option<LanguageIdentifier>,
}

impl HasLanguage for PackageDescription {
    fn language(&self) -> &Option<LanguageIdentifier> {
        &self.language
    }
}

#[derive(Debug, Default)]
pub struct License {
    pub value: String,
    pub url: Option<url::Url>,
}

impl TryFrom<&str> for License {
    type Error = Box<dyn std::error::Error>;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let caps = LICENSE_REGEX.captures(s).ok_or("Failed to match license")?;

        let mut value = caps
            .get(1)
            .ok_or("No license text found")?
            .as_str()
            .to_owned();

        let url = caps.get(2).and_then(|url| url.as_str().parse().ok());

        if value.is_empty() && url.is_none() {
            return Err("No license information found".into());
        } else if value.is_empty() {
            value = url.as_ref().map(ToString::to_string).unwrap();
        }

        Ok(License { value, url })
    }
}

#[derive(Debug)]
pub struct Compatibility(u16);

impl TryFrom<&str> for Compatibility {
    type Error = Box<dyn std::error::Error>;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // See <https://github.com/WoltLab/WCF/blob/5.4.2/wcfsetup/install/files/lib/system/package/PackageUpdateDispatcher.class.php#L497>
        static REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^(?:201[7-9]|20[2-9][0-9])$").unwrap());

        if REGEX.is_match(s) {
            return Ok(Self(s.parse().unwrap()));
        }

        Err(format!("Invalid API version: {}", s).into())
    }
}

impl Display for Compatibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Default)]
pub struct PackageInformation {
    pub name: Vec<PackageName>,
    pub description: Vec<PackageDescription>,
    pub url: Option<url::Url>,
    pub is_application: bool,
    pub version: Version,
    pub date: String,
    pub license: Option<License>,
}

#[derive(Debug, Default)]
pub struct AuthorInformation {
    pub author: String,
    pub author_url: Option<String>,
}

#[derive(Debug, Default)]
pub struct RequiredPackage {
    pub identifier: String,
    pub min_version: String,
}

#[derive(Debug, Default)]
pub struct OptionalPackage {
    pub identifier: String,
}

#[derive(Debug, Default)]
pub struct ExcludedPackage {
    pub identifier: String,
    pub version: Option<String>,
}

#[derive(Debug, Default)]
pub struct UpdateInstruction {
    pub from_version: String,
}

#[derive(Debug, Default)]
pub struct PackageXML {
    pub name: String,

    pub package_information: PackageInformation,
    pub author_information: AuthorInformation,
    pub required_packages: Vec<RequiredPackage>,
    pub optional_packages: Vec<OptionalPackage>,
    pub excluded_packages: Vec<ExcludedPackage>,
    pub instructions: Vec<UpdateInstruction>,

    /// Since API version 3.1
    pub compatibility: Vec<Compatibility>,
}
