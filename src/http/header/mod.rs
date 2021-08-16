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

use actix_web::{
    http::header::{EntityTag, Header, HttpDate, IfModifiedSince, IfNoneMatch, IF_NONE_MATCH},
    web::{Bytes, BytesMut, PayloadConfig},
    FromRequest, HttpMessage, HttpRequest,
};
use askama_actix::futures;
use fluent_templates::LanguageIdentifier;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod accept_language;
pub use accept_language::AcceptLanguage;

#[derive(Debug)]
pub struct Language(pub LanguageIdentifier);

pub fn negotiate_language(requested: &[&LanguageIdentifier]) -> LanguageIdentifier {
    let supported = fluent_langneg::negotiate_languages(
        requested,
        &crate::fluent::AVAILABLE,
        Some(&crate::fluent::DEFAULT),
        fluent_langneg::NegotiationStrategy::Filtering,
    );

    (*supported.first().unwrap()).clone()
}

impl std::ops::Deref for Language {
    type Target = LanguageIdentifier;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&HttpRequest> for Language {
    fn from(req: &HttpRequest) -> Language {
        match AcceptLanguage::parse(req) {
            Ok(mut accept) => {
                let requested = accept.as_mut_slice();
                requested.sort_by(|a, b| b.quality.cmp(&a.quality));

                let requested = requested.iter_mut().map(|q| &q.item).collect::<Vec<_>>();
                let language = negotiate_language(&requested);

                Self(language)
            }
            Err(err) => {
                log::error!("Failed to parse Accept-Language header: {:?}", err);
                Self(crate::fluent::DEFAULT.clone())
            }
        }
    }
}

impl FromRequest for Language {
    type Config = PayloadConfig;
    type Error = actix_web::error::Error;
    type Future = futures::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &HttpRequest,
        _: &mut actix_web::dev::Payload,
    ) -> <Self as FromRequest>::Future {
        futures::ready(Ok(req.into()))
    }
}

#[derive(Debug)]
pub struct Host(String);

impl FromRequest for Host {
    type Config = PayloadConfig;
    type Error = actix_web::error::Error;
    type Future = futures::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &HttpRequest,
        _: &mut actix_web::dev::Payload,
    ) -> <Self as FromRequest>::Future {
        futures::ready(if let Some(host) = &crate::SETTINGS.host {
            Ok(Self(host.clone()))
        } else {
            let info = req.connection_info();

            Ok(Self(format!("{}://{}", info.scheme(), info.host())))
        })
    }
}

impl std::ops::Deref for Host {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(self) struct Writer {
    buf: BytesMut,
}

impl Writer {
    fn new() -> Writer {
        Writer {
            buf: BytesMut::new(),
        }
    }
    fn take(&mut self) -> Bytes {
        self.buf.split().freeze()
    }
}

impl std::fmt::Write for Writer {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buf.extend_from_slice(s.as_bytes());
        Ok(())
    }

    #[inline]
    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        std::fmt::write(self, args)
    }
}

/// Returns true if `req` doesn't have an `If-None-Match` header matching `req`.
///
/// Copyright (c) 2017 Actix Team
/// See [actix_files](https://github.com/actix/actix-web/blob/web-v3.3.2/actix-files/src/named.rs#L476-L495).
pub fn none_match(etag: Option<&EntityTag>, req: &HttpRequest) -> bool {
    match req.get_header::<IfNoneMatch>() {
        Some(IfNoneMatch::Any) => false,

        Some(IfNoneMatch::Items(ref items)) => {
            if let Some(some_etag) = etag {
                if items.iter().any(|item| item.weak_eq(some_etag)) {
                    return false;
                }
            }

            true
        }

        None => true,
    }
}

/// Returns `true` if the resource is considered to not have been modified.
///
/// Copyright (c) 2017 Actix Team
/// See [actix-files](https://github.com/actix/actix-web/blob/web-v3.3.2/actix-files/src/named.rs#L342-L359)
pub fn not_modified(
    req: &HttpRequest,
    etag: Option<&EntityTag>,
    last_modified: Option<HttpDate>,
) -> bool {
    // check last modified
    if !none_match(etag, req) {
        return true;
    }

    if req.headers().contains_key(IF_NONE_MATCH) {
        return false;
    }

    if let (Some(ref m), Some(IfModifiedSince(ref since))) = (last_modified, req.get_header()) {
        let t1 = SystemTime::from(*m).duration_since(UNIX_EPOCH);
        let t2 = SystemTime::from(*since).duration_since(UNIX_EPOCH);

        if let (Ok(t1), Ok(t2)) = (t1, t2) {
            return t1 <= t2;
        }
    }

    false
}
