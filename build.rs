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

use cargo_license::GetDependenciesOpt;
use sha2::{Digest, Sha384};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn write_license_info_file(
    dependencies: Vec<cargo_license::DependencyDetails>,
) -> std::io::Result<()> {
    let dst = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("licence_info.rs");
    let mut file = std::fs::File::create(dst)?;

    writeln!(file, "pub type LicenseInfo = &'static [(&'static str, &'static str, &'static str, &'static [&'static str])];")?;
    writeln!(file, "pub const LICENSE_INFO: LicenseInfo = &[")?;

    for dependency in dependencies {
        let name = dependency.name.clone();
        let version = dependency.version.clone();
        let license = dependency.license.unwrap_or_else(|| "N/A".to_owned());
        let authors = dependency
            .authors
            .unwrap_or_else(|| "N/A".to_owned())
            .split('|')
            .map(|a| format!(r#""{}""#, a.replace(" <>", "")))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(
            file,
            r#"    ("{name}", "{version}", "{license}", &[{authors}]),"#
        )?;
    }

    writeln!(file, "];")?;

    Ok(())
}

fn bundle_source_code() -> crate::Result<()> {
    use ignore::{overrides::OverrideBuilder, WalkBuilder};
    use std::io::BufWriter;

    fn const_name(path: &Path, const_names: &mut HashMap<PathBuf, String>) -> String {
        if let Some(name) = const_names.get(path) {
            return name.clone();
        }

        let name = format!("FILE_{}", const_names.len());

        const_names.insert(path.to_path_buf(), name.clone());

        name
    }

    let mut builder = phf_codegen::OrderedMap::new();

    let overrides = OverrideBuilder::new("./")
        .add("!/.git/")
        .unwrap()
        .add("!/target/")
        .unwrap()
        .build()
        .unwrap();

    let walker = WalkBuilder::new("./")
        .hidden(false)
        .follow_links(true)
        .require_git(false)
        .overrides(overrides)
        .sort_by_file_name(|a, b| a.cmp(b))
        .build();

    let dst = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("source_files.rs");
    let mut out = BufWriter::new(std::fs::File::create(dst)?);

    let mut const_names = HashMap::new();

    let mut combined_hasher = Sha384::new();

    for result in walker {
        let entry = result?;
        if let Some(file_type) = entry.file_type() {
            if !file_type.is_file() {
                continue;
            }

            println!("cargo:rerun-if-changed={}", entry.path().display());

            let file = entry.path().strip_prefix("./").unwrap();
            let name = file.to_str().unwrap().to_owned();
            let const_name = const_name(file, &mut const_names);

            let hash = {
                let mut hasher = Sha384::new();
                let mut file = std::fs::File::open(entry.path())?;
                let _ = std::io::copy(&mut file, &mut hasher)?;
                hasher.finalize()
            };

            combined_hasher.update(hash);

            let src = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join(entry.path());

            writeln!(
                out,
                "const {}: &[u8] = include_bytes!(\"{}\");",
                const_name,
                src.display()
            )?;

            writeln!(
                out,
                "const {}_SHA384_DIGEST: &str = \"sha384-{}\";",
                const_name,
                base64::encode(hash)
            )?;

            builder.entry(
                name,
                &format!(
                    "SourceFile {{ contents: {}, sha384_digest: {}_SHA384_DIGEST }}",
                    &const_name, &const_name
                ),
            );
        }
    }

    writeln!(
        out,
        "pub const SOURCE_FILES: ::phf::OrderedMap<&'static str, SourceFile> = \n{};\n",
        builder.build()
    )?;

    writeln!(
        out,
        "pub const SOURCE_FILES_COMBINED_HASH: &str = \"{}\";\n",
        base64::encode(combined_hasher.finalize())
    )?;

    out.flush().unwrap();

    Ok(())
}

fn main() -> crate::Result<()> {
    built::write_built_file().expect("Failed to acquire build-time information");

    let cmd = Default::default();

    let ops = GetDependenciesOpt {
        avoid_dev_deps: false,
        avoid_build_deps: false,
        direct_deps_only: false,
        root_only: false,
    };

    let dependencies = cargo_license::get_dependencies_from_cargo_lock(cmd, ops)?;

    write_license_info_file(dependencies)?;

    bundle_source_code()?;

    Ok(())
}
