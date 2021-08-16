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

use ::actix_web::http::header::{ETag, EntityTag};

#[derive(Debug)]
pub struct SourceFile {
    pub contents: &'static [u8],
    pub sha384_digest: &'static str,
}

impl From<&SourceFile> for ETag {
    fn from(source_file: &SourceFile) -> ETag {
        ETag(EntityTag::new(false, source_file.sha384_digest.to_owned()))
    }
}

include!(concat!(env!("OUT_DIR"), "/source_files.rs"));
