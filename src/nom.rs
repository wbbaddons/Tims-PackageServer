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

use nom::{
    character::complete::{digit1, multispace0},
    error::ParseError,
    sequence::delimited,
    IResult,
};

/// Parses a numeric value and returns it as T.
pub fn numeric<T: std::str::FromStr>(input: &str) -> IResult<&str, T> {
    let (input, number) = digit1(input)?;

    match number.parse() {
        Ok(value) => Ok((input, value)),
        Err(err) => Err(nom::Err::Error(
            nom::error::FromExternalError::from_external_error(
                input,
                nom::error::ErrorKind::Digit,
                err,
            ),
        )),
    }
}

#[test]
fn test_numeric() {
    assert_eq!(numeric("10"), Ok(("", 10)));
    assert_eq!(numeric("943587"), Ok(("", 943587)));
    assert_eq!(numeric("345fg"), Ok(("fg", 345)));
    assert!(numeric::<u32>("abc123").is_err());
}

/// Ignores whitespaces around the `inner` parser.
pub fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}
