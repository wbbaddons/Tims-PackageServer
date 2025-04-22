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

mod suffix;
mod suffix_type;

pub use suffix::Suffix;
pub use suffix_type::SuffixType;

use crate::nom::numeric;
use nom::{
    character::complete::{char, multispace0},
    combinator::{complete, eof, opt},
    sequence::terminated,
    IResult, Parser,
};
use std::{cmp::Ordering, fmt::Display};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    suffix: Option<Suffix>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32, suffix: Option<Suffix>) -> Self {
        Self {
            major,
            minor,
            patch,
            suffix,
        }
    }

    pub fn major(&self) -> u32 {
        self.major
    }

    pub fn minor(&self) -> u32 {
        self.minor
    }

    pub fn patch(&self) -> u32 {
        self.patch
    }

    pub fn suffix(&self) -> Option<Suffix> {
        self.suffix
    }

    pub fn format_url(&self) -> String {
        if let Some(suffix) = self.suffix {
            format!(
                "{}.{}.{}_{}",
                self.major,
                self.minor,
                self.patch,
                suffix.format_url()
            )
        } else {
            self.to_string()
        }
    }

    pub fn parser(input: &str) -> IResult<&str, Version> {
        parser(input)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(suffix) = self.suffix {
            write!(f, " {}", suffix)?;
        }

        Ok(())
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let major = self.major.cmp(&other.major);
        if major != Ordering::Equal {
            return major;
        }

        let minor = self.minor.cmp(&other.minor);
        if minor != Ordering::Equal {
            return minor;
        }

        let patch = self.patch.cmp(&other.patch);
        if patch != Ordering::Equal {
            return patch;
        }

        let ((weight_1, number_1), (weight_2, number_2)) = match (self.suffix, other.suffix) {
            (Some(s1), Some(s2)) => (
                (s1.ty().weight(), s1.number()),
                (s2.ty().weight(), s2.number()),
            ),
            (None, Some(s2)) => ((0, 0), (s2.ty().weight(), s2.number())),
            (Some(s1), None) => ((s1.ty().weight(), s1.number()), (0, 0)),
            (None, None) => ((0, 0), (0, 0)),
        };

        let suffix_type = weight_1.cmp(&weight_2);
        if suffix_type != Ordering::Equal {
            return suffix_type;
        }

        let suffix_number = number_1.cmp(&number_2);
        if suffix_number != Ordering::Equal {
            return suffix_number;
        }

        Ordering::Equal
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> TryFrom<&'a str> for Version {
    type Error = nom::Err<nom::error::Error<&'a str>>;

    fn try_from(other: &'a str) -> Result<Self, Self::Error> {
        terminated(parser, eof).parse(other).map(|(rest, version)| {
            assert!(rest.is_empty());

            version
        })
    }
}

/// Parses valid version numbers.
fn parser(input: &str) -> IResult<&str, Version> {
    let (input, _) = multispace0(input)?;
    let (input, major) = numeric(input)?;
    let (input, _) = char('.')(input)?;
    let (input, minor) = numeric(input)?;
    let (input, _) = char('.')(input)?;
    let (input, patch) = numeric(input)?;
    let (input, _) = multispace0(input)?;

    let (input, suffix) = opt(complete(Suffix::parser)).parse(input)?;

    Ok((
        input,
        Version {
            major,
            minor,
            patch,
            suffix,
        },
    ))
}

#[test]
fn test_parser() {
    assert_eq!(parser("1.0.0"), Ok(("", Version::new(1, 0, 0, None))));

    assert_eq!(parser("2.0.0"), Ok(("", Version::new(2, 0, 0, None))));

    assert_eq!(parser("1.2.3"), Ok(("", Version::new(1, 2, 3, None))));

    assert_eq!(
        parser("2021.07.21"),
        Ok(("", Version::new(2021, 7, 21, None)))
    );

    assert_eq!(
        parser("13.3.7 pl 42"),
        Ok((
            "",
            Version::new(13, 3, 7, Some(Suffix::new(SuffixType::PatchLevel, 42)))
        ))
    );

    assert_eq!(
        parser("1.0.0 Foobar 4"),
        Ok(("Foobar 4", Version::new(1, 0, 0, None)))
    );

    assert!(parser("1 0 0").is_err());
    assert!(parser("1.0_0").is_err());
    assert!(parser("1-0-0").is_err());
    assert!(parser("1.0").is_err());
}
