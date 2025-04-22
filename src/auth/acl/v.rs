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

use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{map, value},
    IResult, Parser,
};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum V {
    VersionInput,
    Version(Version),
}

impl V {
    pub fn parser(input: &str) -> IResult<&str, V> {
        parser(input)
    }
}

/// Parses either `$v` or a valid version number.
fn parser(input: &str) -> IResult<&str, V> {
    alt((
        value(V::VersionInput, tag("$v")),
        map(Version::parser, V::Version),
    ))
    .parse(input)
}

#[test]
fn test_parser() {
    use crate::version::{Suffix, SuffixType};

    assert_eq!(parser("$v"), Ok(("", V::VersionInput)));
    assert_eq!(parser("$v >= 1.0.0"), Ok((" >= 1.0.0", V::VersionInput)));
    assert_eq!(
        parser("13.3.7 pl 42"),
        Ok((
            "",
            V::Version(Version::new(
                13,
                3,
                7,
                Some(Suffix::new(SuffixType::PatchLevel, 42))
            ))
        ))
    );
    assert_eq!(
        parser("1.0.0"),
        Ok(("", V::Version(Version::new(1, 0, 0, None))))
    );
    assert_eq!(
        parser("2.0.0"),
        Ok(("", V::Version(Version::new(2, 0, 0, None))))
    );
}
