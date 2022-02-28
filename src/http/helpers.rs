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
    auth::{AuthData, Permissions},
    version::Version,
};
use actix_web::{
    http::{header::LOCATION, StatusCode},
    HttpResponse,
};
use actix_web_httpauth::extractors::basic::BasicAuth;

pub fn is_accessible(
    package_id: &str,
    version: &Version,
    auth_info: &AuthInfo,
    auth_data: &AuthData,
) -> bool {
    let check_permissions = |permissions: &Permissions| -> bool {
        for (name_regex, rule) in permissions {
            if name_regex.0.is_match(package_id) && rule.evaluate(version) {
                return true;
            }
        }

        false
    };

    // First check the general package rules
    if check_permissions(&auth_data.packages) {
        return true;
    }

    // Then check the user’s permissions
    if let Some(username) = &auth_info.username {
        if let Some(user_data) = auth_data.users.get(username) {
            // Check the user’s own package permissions
            if check_permissions(&user_data.packages) {
                return true;
            }

            // Check the user’s groups
            for group in &user_data.groups {
                if let Some(group) = auth_data.groups.get(group) {
                    if check_permissions(group) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

#[derive(Debug)]
pub struct AuthInfo {
    pub username: Option<String>,
}

pub fn get_auth_info(auth_data: &AuthData, auth: Option<BasicAuth>) -> AuthInfo {
    let username = auth.and_then(|auth| {
        let user_id = auth.user_id();
        let password = auth.password();

        let user = auth_data.users.get(user_id.as_ref());

        user.and_then(|user| match password {
            Some(password) if user.passwd.verify(password) => Some(user_id.as_ref().to_owned()),
            _ => None,
        })
    });

    AuthInfo { username }
}

#[allow(unused)]
#[derive(Debug)]
pub enum RedirectType {
    Temporary(String),
    Permanent(String),

    Moved(String),
    Found(String),
    Other(String),
}

impl RedirectType {
    fn to(self) -> String {
        match self {
            Self::Temporary(to)
            | Self::Permanent(to)
            | Self::Moved(to)
            | Self::Found(to)
            | Self::Other(to) => to,
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Temporary(..) => StatusCode::TEMPORARY_REDIRECT,
            Self::Permanent(..) => StatusCode::PERMANENT_REDIRECT,

            Self::Moved(..) => StatusCode::MOVED_PERMANENTLY,
            Self::Found(..) => StatusCode::FOUND,
            Self::Other(..) => StatusCode::SEE_OTHER,
        }
    }
}

pub fn redirect(ty: RedirectType) -> HttpResponse {
    HttpResponse::build(ty.status_code())
        .insert_header((LOCATION, ty.to()))
        .finish()
}
