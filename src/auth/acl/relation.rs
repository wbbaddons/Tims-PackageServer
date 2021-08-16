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

use crate::nom::ws;

use nom::{branch::alt, bytes::complete::tag, combinator::value, IResult};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Relation {
    GreaterOrEquals,
    Greater,
    LessOrEquals,
    Less,
}

impl Relation {
    pub fn parser(input: &str) -> IResult<&str, Relation> {
        parser(input)
    }
}

/// Parses a relational operator.
fn parser(input: &str) -> IResult<&str, Relation> {
    ws(alt((
        value(Relation::LessOrEquals, tag("<=")),
        value(Relation::Less, tag("<")),
        value(Relation::GreaterOrEquals, tag(">=")),
        value(Relation::Greater, tag(">")),
    )))(input)
}

#[test]
fn test_parser() {
    assert_eq!(parser("<="), Ok(("", Relation::LessOrEquals)));
    assert_eq!(parser("<"), Ok(("", Relation::Less)));
    assert_eq!(parser(">="), Ok(("", Relation::GreaterOrEquals)));
    assert_eq!(parser(">"), Ok(("", Relation::Greater)));

    assert_eq!(parser("<= 1.0.0"), Ok(("1.0.0", Relation::LessOrEquals)));
    assert_eq!(parser("< 1.0.0"), Ok(("1.0.0", Relation::Less)));
    assert_eq!(parser(">= 1.0.0"), Ok(("1.0.0", Relation::GreaterOrEquals)));
    assert_eq!(parser("> 1.0.0"), Ok(("1.0.0", Relation::Greater)));

    assert!(parser("==").is_err());
    assert!(parser("!=").is_err());
    assert!(parser("~").is_err());
    assert!(parser("1.0.0 >= 0.0.1").is_err());
}
