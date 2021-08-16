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

//! The built-in [`actix_web::http::header::AcceptLanguage`] parses into [`actix_web::http::header::LanguageTag`],
//! but we need to parse into [`fluent_templates::LanguageIdentifier`].

use super::Writer;
use actix_web::http::header::{
    fmt_comma_delimited, from_comma_delimited, Header, HeaderName, HeaderValue, IntoHeaderValue,
    InvalidHeaderValue, QualityItem, ACCEPT_LANGUAGE,
};
use fluent_templates::LanguageIdentifier;

pub type Output = Vec<QualityItem<LanguageIdentifier>>;

#[derive(Clone, Debug, PartialEq)]
pub struct AcceptLanguage(pub Output);

impl ::std::ops::Deref for AcceptLanguage {
    type Target = Output;

    #[inline]
    fn deref(&self) -> &Output {
        &self.0
    }
}

impl ::std::ops::DerefMut for AcceptLanguage {
    #[inline]
    fn deref_mut(&mut self) -> &mut Output {
        &mut self.0
    }
}

impl Header for AcceptLanguage {
    #[inline]
    fn name() -> HeaderName {
        ACCEPT_LANGUAGE
    }

    #[inline]
    fn parse<T>(msg: &T) -> Result<Self, actix_web::error::ParseError>
    where
        T: actix_web::HttpMessage,
    {
        from_comma_delimited(msg.headers().get_all(Self::name())).map(AcceptLanguage)
    }
}

impl std::fmt::Display for AcceptLanguage {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        fmt_comma_delimited(f, &self.0[..])
    }
}

impl IntoHeaderValue for AcceptLanguage {
    type Error = InvalidHeaderValue;

    fn try_into(self) -> Result<HeaderValue, Self::Error> {
        use std::fmt::Write;
        let mut writer = Writer::new();
        let _ = write!(&mut writer, "{}", self);
        HeaderValue::from_maybe_shared(writer.take())
    }
}
