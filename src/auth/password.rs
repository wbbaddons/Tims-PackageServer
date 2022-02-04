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

use super::hashers::{BannedUser, Bcrypt, DoubleBcrypt, UnknownHash};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{de::Visitor, Deserialize, Deserializer};
use std::fmt::Debug;

pub trait PasswordHash: Debug + Send + Sync {
    fn verify(&self, _password: &str) -> bool {
        false
    }
}

struct PasswordHashVisitor;
impl<'de> Visitor<'de> for PasswordHashVisitor {
    type Value = Box<dyn PasswordHash>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid password hash")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        static LEGACY_DOUBLE_BCRYPT_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r#"^\$2[afxy]\$"#).unwrap());

        if s == "-" {
            return Ok(Box::new(BannedUser));
        }

        if LEGACY_DOUBLE_BCRYPT_REGEX.is_match(s) {
            return Ok(Box::new(DoubleBcrypt(s.to_owned())));
        }

        let mut split = s.splitn(2, ':');
        if let (Some(name), Some(hash)) = (split.next(), split.next()) {
            assert_eq!(None, split.next());

            let name = name.to_ascii_lowercase();

            if name == "bcrypt" {
                return Ok(Box::new(Bcrypt(hash.to_owned())));
            }
        }

        Ok(Box::new(UnknownHash(s.to_owned())))
    }
}

impl<'de> Deserialize<'de> for Box<dyn PasswordHash> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(PasswordHashVisitor)
    }
}
