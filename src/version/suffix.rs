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

use super::SuffixType;
use crate::nom::{numeric, ws};
use nom::{combinator::map, sequence::tuple, IResult};

use serde::Serialize;
use std::fmt::Display;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize)]
pub struct Suffix {
    ty: SuffixType,
    number: u32,
}

impl Suffix {
    pub fn new(ty: SuffixType, number: u32) -> Self {
        Self { ty, number }
    }

    pub fn number(self) -> u32 {
        self.number
    }

    pub fn ty(self) -> SuffixType {
        self.ty
    }

    pub fn format_url(self) -> String {
        let suffix = match self.ty {
            SuffixType::Alpha => "alpha",
            SuffixType::Beta => "beta",
            SuffixType::Dev => "dev",
            SuffixType::ReleaseCandidate => "rc",
            SuffixType::PatchLevel => "pl",
        };

        format!("{}_{}", suffix, self.number)
    }

    pub fn parser(input: &str) -> IResult<&str, Suffix> {
        parser(input)
    }
}

impl Display for Suffix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let suffix = match self.ty {
            SuffixType::Alpha => "Alpha",
            SuffixType::Beta => "Beta",
            SuffixType::Dev => "Dev",
            SuffixType::ReleaseCandidate => "RC",
            SuffixType::PatchLevel => "pl",
        };

        write!(f, "{} {}", suffix, self.number)
    }
}

/// Parses suffix strings followed by a number into a `Suffix`.
fn parser(input: &str) -> IResult<&str, Suffix> {
    map(
        tuple((ws(SuffixType::parser), ws(numeric))),
        |(ty, number)| Suffix { ty, number },
    )(input)
}

#[test]
fn test_parser() {
    assert!(parser("alpha").is_err());
    assert!(parser("a").is_err());
    assert!(parser("rc").is_err());
    assert!(parser("foo alpha 3").is_err());
    assert!(parser("alberta").is_err());
    assert!(parser("boot").is_err());
    assert!(parser("develop").is_err());
    assert!(parser("Foobar 4").is_err());

    assert_eq!(
        parser("alpha 3"),
        Ok((
            "",
            Suffix {
                ty: SuffixType::Alpha,
                number: 3
            }
        ))
    );
    assert_eq!(
        parser("BeTa 1337"),
        Ok((
            "",
            Suffix {
                ty: SuffixType::Beta,
                number: 1337
            }
        ))
    );
    assert_eq!(
        parser("RC 2"),
        Ok((
            "",
            Suffix {
                ty: SuffixType::ReleaseCandidate,
                number: 2
            }
        ))
    );
    assert_eq!(
        parser("Dev 20210724 Unstable"),
        Ok((
            "Unstable",
            Suffix {
                ty: SuffixType::Dev,
                number: 20210724
            }
        ))
    );
    assert_eq!(
        parser("pl 3 "),
        Ok((
            "",
            Suffix {
                ty: SuffixType::PatchLevel,
                number: 3
            }
        ))
    );
}
