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

use serde::{de::Visitor, Deserialize, Deserializer};

use super::expression::Expression;
use crate::{nom::ws, version::Version};
use nom::{
    branch::alt,
    character::complete::char,
    combinator::{eof, map, value},
    sequence::terminated,
    Parser,
};

#[derive(Debug, Clone)]
pub enum Ruleset {
    Star,
    Expression(Expression),
}

impl Ruleset {
    pub fn evaluate(&self, v: &Version) -> bool {
        match self {
            Ruleset::Star => true,
            Ruleset::Expression(e) => e.evaluate(v),
        }
    }

    pub fn parser(input: &str) -> Result<Ruleset, nom::Err<nom::error::Error<&str>>> {
        parser(input)
    }
}

struct RulesetVisitor;

impl<'de> Visitor<'de> for RulesetVisitor {
    type Value = Ruleset;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid ruleset")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ruleset::parser(s).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for Ruleset {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(RulesetVisitor)
    }
}

/// Parses a valid ruleset for version access control.
fn parser(input: &str) -> Result<Ruleset, nom::Err<nom::error::Error<&str>>> {
    let (rest, ruleset) = terminated(
        alt((
            value(Ruleset::Star, char('*')),
            map(ws(Expression::parser), Ruleset::Expression),
        )),
        eof,
    )
    .parse(input)?;

    assert!(rest.is_empty());

    Ok(ruleset)
}

#[test]
fn test_parser() {
    assert!(parser("*").is_ok());

    assert!(parser("$v ~ beta").is_ok());
    assert!(parser("$v !~ beta").is_ok());
    assert!(parser("$v !~ beta || $v !~ alpha").is_ok());
    assert!(parser("$v ~ beta || $v ~ alpha").is_ok());
    assert!(parser("$v !~ beta || $v ~ alpha").is_ok());
    assert!(parser("$v ~ beta || $v !~ alpha").is_ok());
    assert!(parser("$v !~ beta || $v !~ alpha || $v !~ dev").is_ok());
    assert!(parser("$v !~ beta && $v !~ alpha").is_ok());
    assert!(parser("$v !~ beta && $v !~ alpha && $v !~ dev").is_ok());
    assert!(parser("$v !~ beta && $v !~ alpha || $v !~ dev").is_err());
    assert!(parser("$v !~ beta || $v !~ alpha && $v !~ dev").is_err());
    assert!(parser("($v ~ beta)").is_ok());
    assert!(parser("($v ~ beta || $v ~ alpha)").is_ok());
    assert!(parser("($v~beta||$v~alpha)").is_ok());
    assert!(parser("($v ~ beta || $v ~ alpha) && $v !~ dev").is_ok());
    assert!(parser("($v ~ beta && $v ~ alpha) || $v !~ dev").is_ok());
    assert!(parser("$v = 1.0.0").is_ok());
    assert!(parser("1.0.0 = 2.0.0").is_ok());
    assert!(parser("$v !~ beta && $v !~ alpha && 1.0.0 = 2.0.0").is_ok());
    assert!(parser("($v !~ beta && $v !~ alpha && 1.0.0 = 2.0.0)").is_ok());
    assert!(parser("$v!~beta&&$v!~alpha&&1.0.0=2.0.0").is_ok());
    assert!(parser("($v!~beta&&$v!~alpha&&1.0.0=2.0.0)").is_ok());
}

#[test]
fn test_evaluate() {
    assert!(parser("$v ~ beta")
        .unwrap()
        .evaluate(&Version::parser("1.0.0 Beta 1").unwrap().1));
    assert!(!parser("$v ~ beta")
        .unwrap()
        .evaluate(&Version::parser("1.0.0").unwrap().1));
    assert!(parser("$v !~ beta")
        .unwrap()
        .evaluate(&Version::parser("1.0.0 Alpha 1").unwrap().1));
    assert!(parser("$v !~ beta")
        .unwrap()
        .evaluate(&Version::parser("1.0.0").unwrap().1));

    {
        let rs = parser("($v !~ beta && $v !~ alpha && 1.0.0 != 2.0.0)").unwrap();
        assert!(rs.evaluate(&Version::parser("1.0.0").unwrap().1));
        // Should be evaluable multiple times
        assert!(rs.evaluate(&Version::parser("1.0.0").unwrap().1));
    }

    {
        let rs = parser("1.0.0 <= $v < 2.0.0 || 2.0.0 <= $v < 3.0.0").unwrap();
        assert!(rs.evaluate(&Version::parser("1.0.0").unwrap().1));
        assert!(rs.evaluate(&Version::parser("1.5.1").unwrap().1));
        assert!(rs.evaluate(&Version::parser("2.0.3 alpha 1").unwrap().1));
        assert!(!rs.evaluate(&Version::parser("3.0.0").unwrap().1));
    }

    {
        let rs = parser("1.0.0 <= $v < 2.0.0 && $v != 1.5.1").unwrap();
        assert!(rs.evaluate(&Version::parser("1.0.0").unwrap().1));
        assert!(rs.evaluate(&Version::parser("1.5.0").unwrap().1));
        assert!(!rs.evaluate(&Version::parser("1.5.1").unwrap().1));
        assert!(rs.evaluate(&Version::parser("1.5.2").unwrap().1));
        assert!(!rs.evaluate(&Version::parser("2.0.3 alpha 1").unwrap().1));
        assert!(!rs.evaluate(&Version::parser("3.0.0").unwrap().1));
    }

    {
        let rs = parser("$v = 1.0.0 || $v = 2.0.0 || $v = 3.0.0 || $v = 4.0.0").unwrap();
        assert!(rs.evaluate(&Version::parser("1.0.0").unwrap().1));
        assert!(rs.evaluate(&Version::parser("2.0.0").unwrap().1));
        assert!(rs.evaluate(&Version::parser("3.0.0").unwrap().1));
        assert!(rs.evaluate(&Version::parser("4.0.0").unwrap().1));
        assert!(!rs.evaluate(&Version::parser("5.0.0").unwrap().1));
    }
}
