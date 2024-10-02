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

use super::{acl::Ruleset, password::PasswordHash};
use regex::Regex;
use serde::{de::Visitor, Deserialize, Deserializer};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

#[derive(Debug)]
pub struct PackageName(Regex);

impl PackageName {
    pub fn regex(&self) -> &Regex {
        &self.0
    }
}

struct PackageNameVisitor;
impl<'de> Visitor<'de> for PackageNameVisitor {
    type Value = PackageName;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid package expression")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let regex_str = regex::escape(s).replace("\\*", ".*");
        let regex_str = format!("^{regex_str}$");

        let regex = Regex::new(&regex_str).map_err(serde::de::Error::custom)?;

        Ok(PackageName(regex))
    }
}

impl<'de> Deserialize<'de> for PackageName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(PackageNameVisitor)
    }
}

impl Eq for PackageName {}
impl PartialEq for PackageName {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str().eq(other.0.as_str())
    }
}

impl Hash for PackageName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

type UserName = String;
type GroupName = String;

pub type Permissions = HashMap<PackageName, Ruleset>;

#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub passwd: Box<dyn PasswordHash>,

    #[serde(default)]
    pub groups: Vec<GroupName>,

    #[serde(default)]
    pub packages: Permissions,
}

#[derive(Debug, Default, Deserialize)]
pub struct AuthData {
    #[serde(default)]
    pub users: HashMap<UserName, UserInfo>,

    #[serde(default)]
    pub groups: HashMap<GroupName, Permissions>,

    #[serde(default)]
    pub packages: Permissions,
}
