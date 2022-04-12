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

use crate::auth::PasswordHash;

#[derive(Debug)]
pub struct DoubleBcrypt(pub String);

impl PasswordHash for DoubleBcrypt {
    fn verify(&self, password: &str) -> bool {
        match verify(password, &self.0) {
            Ok(result) => result,
            Err(err) => {
                log::error!("Failed to verify password: {}", err);
                false
            }
        }
    }
}

fn verify(password: &str, hash: &str) -> crate::Result<bool> {
    Ok(password)
        .and_then(|password| get_salted_hash(password, hash))
        .and_then(|salted_hash| bcrypt::verify(&salted_hash, hash).map_err(Into::into))
}

fn get_salted_hash(password: &str, hash: &str) -> crate::Result<String> {
    let version = match hash.split('$').nth(1) {
        Some("2a") => bcrypt::Version::TwoA,
        Some("2x") => bcrypt::Version::TwoX,
        Some("2y") => bcrypt::Version::TwoY,
        Some("2b") => bcrypt::Version::TwoB,
        Some(x) => {
            return Err(format!("Unknown bcrypt version: {}", x).into());
        }
        None => {
            return Err("Failed to parse password hash".into());
        }
    };

    let parts = hash.parse::<bcrypt::HashParts>()?;
    let salt = base64::decode_config(parts.get_salt(), base64::BCRYPT)?;
    let parts = bcrypt::hash_with_salt(password, parts.get_cost(), &salt)?;

    Ok(parts.format_for_version(version))
}

#[test]
fn test_verify() {
    assert!(verify(
        "root",
        "$2a$08$3GNrFLqG5M7BsGI/BtxcGuNWX2iY/UsfTwWnmJiddHB.z/PdkAsR2"
    )
    .unwrap());

    assert!(verify(
        "test",
        "$2a$08$JSycOvMzyJYp86mzTjCeROOLAWel2fibGyE1ILX1Y9ISdeF/pulP."
    )
    .unwrap());
}

#[test]
fn test_verify_method() {
    assert!(DoubleBcrypt(
        "$2a$08$3GNrFLqG5M7BsGI/BtxcGuNWX2iY/UsfTwWnmJiddHB.z/PdkAsR2".to_owned()
    )
    .verify("root"));

    assert!(DoubleBcrypt(
        "$2a$08$JSycOvMzyJYp86mzTjCeROOLAWel2fibGyE1ILX1Y9ISdeF/pulP.".to_owned()
    )
    .verify("test"));
}
