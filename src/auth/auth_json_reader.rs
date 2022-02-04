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

use super::types::AuthData;
use std::{error, fmt, path::PathBuf};

type Result<T> = std::result::Result<T, AuthParseError>;

#[derive(Debug)]
pub enum AuthParseError {
    UnableToOpen(std::io::Error),
    UnableToParse(serde_json::Error),
}

impl fmt::Display for AuthParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AuthParseError::UnableToOpen(..) => {
                write!(f, "Failed to open.")
            }
            AuthParseError::UnableToParse(..) => {
                write!(f, "Failed to parse.")
            }
        }
    }
}

impl error::Error for AuthParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            AuthParseError::UnableToOpen(ref e) => Some(e),
            AuthParseError::UnableToParse(ref e) => Some(e),
        }
    }
}

impl TryFrom<std::path::PathBuf> for AuthData {
    type Error = AuthParseError;

    fn try_from(path: PathBuf) -> std::result::Result<Self, Self::Error> {
        let file = std::fs::File::open(path).map_err(AuthParseError::UnableToOpen)?;

        serde_json::from_reader(file).map_err(AuthParseError::UnableToParse)
    }
}

impl TryFrom<&[u8]> for AuthData {
    type Error = AuthParseError;

    fn try_from(slice: &[u8]) -> std::result::Result<Self, Self::Error> {
        serde_json::from_slice(slice).map_err(AuthParseError::UnableToParse)
    }
}

impl TryFrom<&str> for AuthData {
    type Error = AuthParseError;

    fn try_from(str: &str) -> std::result::Result<Self, Self::Error> {
        serde_json::from_str(str).map_err(AuthParseError::UnableToParse)
    }
}

pub fn read_auth_json(path: std::path::PathBuf) -> Result<AuthData> {
    AuthData::try_from(path)
}

#[test]
fn test_deserialize() {
    let file = crate::SOURCE_FILES
        .get("packages/auth.json.example")
        .unwrap();

    assert!(AuthData::try_from(file.contents).is_ok());
}

#[test]
fn test_read_auth_json() {
    let file = std::path::PathBuf::from("packages/auth.json.example");

    assert!(read_auth_json(file).is_ok());
}

#[test]
fn test_parse() {
    {
        let data = AuthData::try_from(r#"{}"#).unwrap();
        assert_eq!(data.users.len(), 0);
        assert_eq!(data.groups.len(), 0);
        assert_eq!(data.packages.len(), 0);
    }

    {
        let data = AuthData::try_from(
            r#"{
            "users": {},
            "groups": {},
            "packages": {}
        }"#,
        )
        .unwrap();
        assert_eq!(data.users.len(), 0);
        assert_eq!(data.groups.len(), 0);
        assert_eq!(data.packages.len(), 0);
    }

    {
        let data = AuthData::try_from(
            r#"{
            "users": {
                "Foo": {
                    "passwd": "-"
                },
                "Bar": {
                    "passwd": "Bcrypt:$2y$10$0QxMnGyTrXnL7ngq2y/qFui3H2IaEuXfNwbLWR50m9Yarp0HZwEmq"
                },
                "root": {
                    "passwd": "$2a$08$3GNrFLqG5M7BsGI/BtxcGuNWX2iY/UsfTwWnmJiddHB.z/PdkAsR2"
                }
            },
            "groups": {},
            "packages": {}
        }"#,
        )
        .unwrap();

        assert_eq!(data.users.len(), 3);

        assert!(data.users.contains_key("Foo"));
        assert!(!data.users.get("Foo").unwrap().passwd.verify("foo"));
        assert_eq!(data.users.get("Foo").unwrap().groups.len(), 0);

        assert!(data.users.contains_key("Bar"));
        assert!(data.users.get("Bar").unwrap().passwd.verify("bar"));

        assert!(data.users.contains_key("root"));
        assert!(data.users.get("root").unwrap().passwd.verify("root"));

        assert_eq!(data.groups.len(), 0);
        assert_eq!(data.packages.len(), 0);
    }
}
