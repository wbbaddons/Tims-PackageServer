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

use super::{relation::Relation, v::V};
use crate::{
    nom::ws,
    version::{SuffixType, Version},
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::map,
    multi::many1,
    sequence::{delimited, preceded, separated_pair, tuple},
    IResult,
};

#[derive(Debug, Clone)]
pub enum Expression {
    Not(Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Equals(V, V),
    Less(V, V),
    Like(SuffixType),
}

impl Expression {
    pub fn evaluate(&self, v: &Version) -> bool {
        match self {
            Expression::Not(e) => !e.evaluate(v),
            Expression::And(e1, e2) => e1.evaluate(v) && e2.evaluate(v),
            Expression::Or(e1, e2) => e1.evaluate(v) || e2.evaluate(v),

            Expression::Equals(V::VersionInput, V::Version(v2)) => *v == *v2,
            Expression::Equals(V::Version(v1), V::VersionInput) => *v1 == *v,
            Expression::Equals(V::Version(v1), V::Version(v2)) => *v1 == *v2,
            Expression::Equals(_, _) => unreachable!(),

            Expression::Less(V::VersionInput, V::Version(v2)) => *v < *v2,
            Expression::Less(V::Version(v1), V::VersionInput) => *v1 < *v,
            Expression::Less(V::Version(v1), V::Version(v2)) => *v1 < *v2,
            Expression::Less(_, _) => unreachable!(),

            Expression::Like(suffix_type) => {
                if let Some(suffix) = v.suffix() {
                    return suffix.ty() == *suffix_type;
                }

                false
            }
        }
    }

    pub fn parser(input: &str) -> IResult<&str, Expression> {
        parser(input)
    }
}

impl From<(V, Relation, V)> for Expression {
    fn from((v1, r, v2): (V, Relation, V)) -> Self {
        match r {
            Relation::Greater => Expression::Less(v2, v1),
            Relation::GreaterOrEquals => Expression::Not(Box::new(Expression::Less(v1, v2))),
            Relation::Less => Expression::Less(v1, v2),
            Relation::LessOrEquals => Expression::Not(Box::new(Expression::Less(v2, v1))),
        }
    }
}

/// Parses a logical conjunction: Several sub expressions combined using `&&`.
fn and(input: &str) -> IResult<&str, Expression> {
    map(
        tuple((
            ws(sub_expression),
            many1(preceded(ws(tag("&&")), sub_expression)),
        )),
        |(e1, e_list)| {
            assert!(!e_list.is_empty());

            let e2 = e_list
                .into_iter()
                .reduce(|a, b| Expression::And(Box::new(a), Box::new(b)))
                .unwrap();

            Expression::And(Box::new(e1), Box::new(e2))
        },
    )(input)
}

/// Parses a logical disjunction: Several sub expressions combined using `||`.
fn or(input: &str) -> IResult<&str, Expression> {
    map(
        tuple((
            ws(sub_expression),
            many1(preceded(ws(tag("||")), sub_expression)),
        )),
        |(e1, e_list)| {
            let e2 = e_list
                .into_iter()
                .reduce(|a, b| Expression::Or(Box::new(a), Box::new(b)))
                .unwrap();

            Expression::Or(Box::new(e1), Box::new(e2))
        },
    )(input)
}

/// Parses valid sub expressions:
/// - An expression within parentheses.
/// - `$v ~ <suffix_type>`
/// - `$v !~ <suffix_type>`
/// - `<v> != <v>`
/// - `<v> <relation> <v> <relation> <v>`
/// - `<v> <relation> <v>`
/// - `<v> = <v>`
fn sub_expression(input: &str) -> IResult<&str, Expression> {
    alt((
        delimited(char('('), parser, char(')')),
        map(
            separated_pair(tag("$v"), ws(char('~')), SuffixType::parser),
            |(_v, suffix)| Expression::Like(suffix),
        ),
        map(
            separated_pair(tag("$v"), ws(tag("!~")), SuffixType::parser),
            |(_v, suffix)| Expression::Not(Box::new(Expression::Like(suffix))),
        ),
        map(
            separated_pair(V::parser, ws(tag("!=")), V::parser),
            |(v1, v2)| Expression::Not(Box::new(Expression::Equals(v1, v2))),
        ),
        map(
            tuple((
                V::parser,
                ws(Relation::parser),
                V::parser,
                ws(Relation::parser),
                V::parser,
            )),
            |(v1, r1, v2, r2, v3)| {
                let left = (v1, r1, v2).into();
                let right = (v2, r2, v3).into();

                Expression::And(Box::new(left), Box::new(right))
            },
        ),
        map(
            tuple((V::parser, ws(Relation::parser), V::parser)),
            Into::into,
        ),
        map(
            separated_pair(V::parser, ws(char('=')), V::parser),
            |(v1, v2)| Expression::Equals(v1, v2),
        ),
    ))(input)
}

/// Parses a valid expression:
/// - A subexpression.
/// - A logical conjunction (and).
/// - A logical disjunection (or).
fn parser(input: &str) -> IResult<&str, Expression> {
    alt((and, or, sub_expression))(input)
}
