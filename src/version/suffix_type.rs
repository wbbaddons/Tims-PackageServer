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

use nom::{branch::alt, bytes::complete::tag_no_case, combinator::value, IResult, Parser};

use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize)]
pub enum SuffixType {
    Alpha,
    Beta,
    Dev,
    ReleaseCandidate,
    PatchLevel,
}

impl SuffixType {
    pub fn weight(self) -> i8 {
        match self {
            SuffixType::Alpha => -3,
            SuffixType::Beta => -2,
            SuffixType::Dev => -4,
            SuffixType::ReleaseCandidate => -1,
            SuffixType::PatchLevel => 1,
        }
    }

    pub fn parser(input: &str) -> IResult<&str, SuffixType> {
        parser(input)
    }
}

/// Parses suffix strings into the appropriate `SuffixType`.
fn parser(input: &str) -> IResult<&str, SuffixType> {
    alt((
        value(SuffixType::Alpha, tag_no_case("alpha")),
        value(SuffixType::Alpha, tag_no_case("a")),
        value(SuffixType::Beta, tag_no_case("beta")),
        value(SuffixType::Beta, tag_no_case("b")),
        value(SuffixType::Dev, tag_no_case("dev")),
        value(SuffixType::Dev, tag_no_case("d")),
        value(SuffixType::ReleaseCandidate, tag_no_case("rc")),
        value(SuffixType::PatchLevel, tag_no_case("pl")),
    ))
    .parse(input)
}

#[test]
fn test_parser() {
    assert_eq!(parser("alpha"), Ok(("", SuffixType::Alpha)));
    assert_eq!(parser("a"), Ok(("", SuffixType::Alpha)));
    assert_eq!(parser("rc"), Ok(("", SuffixType::ReleaseCandidate)));
    assert_eq!(parser("alpha 3"), Ok((" 3", SuffixType::Alpha)));
    assert_eq!(parser("alpha 3)"), Ok((" 3)", SuffixType::Alpha)));
    assert_eq!(parser("alpha)"), Ok((")", SuffixType::Alpha)));
    assert_eq!(parser("alberta"), Ok(("lberta", SuffixType::Alpha)));
    assert_eq!(parser("boot"), Ok(("oot", SuffixType::Beta)));
    assert_eq!(parser("develop"), Ok(("elop", SuffixType::Dev)));
    assert_eq!(parser("derp"), Ok(("erp", SuffixType::Dev)));

    assert!(parser("foo alpha 3").is_err());
}
