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

use crate::{fluent, http::header::Language, version::Version};
use actix_web::{
    http::{
        header,
        header::{Accept, CacheControl, CacheDirective},
        StatusCode,
    },
    HttpRequest, HttpResponse, HttpResponseBuilder, ResponseError,
};
use actix_web_httpauth::headers::www_authenticate::{basic::Basic as Challenge, WwwAuthenticate};
use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    // Generic 404
    FileNotFound(HttpRequest, String),

    // 503
    PackageListUnavailable(HttpRequest),

    // 406 - The requested accept type cannot be handled
    NotAcceptable(HttpRequest, Accept),

    // User requested a non-existant package
    UnknownPackage(HttpRequest, String),

    // User requested a non-existant package version
    UnknownPackageVersion(HttpRequest, String, Version),

    // Failed to read a known package from the file system
    PackageReadFailed(HttpRequest, String),

    PaymentRequired(HttpRequest, String, Version),

    AccessDenied(HttpRequest),

    IoError(HttpRequest, std::io::Error),
}

impl Error {
    fn get_request(&self) -> &HttpRequest {
        match *self {
            Self::AccessDenied(ref req)
            | Self::FileNotFound(ref req, ..)
            | Self::NotAcceptable(ref req, ..)
            | Self::UnknownPackage(ref req, ..)
            | Self::UnknownPackageVersion(ref req, ..)
            | Self::PackageReadFailed(ref req, ..)
            | Self::PaymentRequired(ref req, ..)
            | Self::PackageListUnavailable(ref req)
            | Self::IoError(ref req, ..) => req,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let req = self.get_request();
        let lang = Language::from(req);

        let message = match self {
            Self::AccessDenied(..) => fluent!(lang, "password-prompt"),

            Self::FileNotFound(_, ref file) => fluent!(lang, "file-not-found", { file }),
            Self::NotAcceptable(_, ref accept) => {
                fluent!(lang, "unacceptable-accept-type", { "value": accept.to_string() })
            }

            Self::UnknownPackage(_, ref package_id) => {
                fluent!(lang, "package-unknown", { package_id })
            }
            Self::UnknownPackageVersion(_, ref package_id, version) => {
                fluent!(lang, "package-unknown-version", { package_id, "version": version.to_string() })
            }
            Self::PackageReadFailed(_, ref file) => {
                fluent!(lang, "package-read-failed", { file })
            }
            Self::PaymentRequired(_, ref package_id, version) => {
                fluent!(lang, "package-payment-required", { package_id, "version": version.to_string() })
            }
            Self::PackageListUnavailable(..) => fluent!(lang, "package-list-unavailable"),
            Self::IoError(..) => panic!("Not implemented"),
        };

        f.write_str(&message)
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::AccessDenied(..) => StatusCode::UNAUTHORIZED,
            Self::FileNotFound(..) => StatusCode::NOT_FOUND,
            Self::NotAcceptable(..) => StatusCode::NOT_ACCEPTABLE,

            Self::UnknownPackage(..) => StatusCode::NOT_FOUND,
            Self::UnknownPackageVersion(..) => StatusCode::NOT_FOUND,
            Self::PackageReadFailed(..) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::PaymentRequired(..) => StatusCode::PAYMENT_REQUIRED,

            Self::PackageListUnavailable(..) => StatusCode::SERVICE_UNAVAILABLE,
            Self::IoError(_, e) => e.status_code(),
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::IoError(_, e) = self {
            return e.error_response();
        }

        if let Self::AccessDenied(ref req) = self {
            let lang = Language::from(req);
            let challenge = Challenge::with_realm(fluent!(lang, "password-prompt"));

            return HttpResponse::Unauthorized()
                .insert_header(WwwAuthenticate(challenge))
                .insert_header(CacheControl(vec![
                    CacheDirective::NoCache,
                    CacheDirective::NoStore,
                    CacheDirective::Private,
                ]))
                .body(fluent!(lang, "access-denied"));
        }

        HttpResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, "text/plain; charset=utf-8"))
            .insert_header(CacheControl(vec![
                CacheDirective::NoCache,
                CacheDirective::NoStore,
                CacheDirective::Private,
            ]))
            .body(self.to_string())
    }
}
