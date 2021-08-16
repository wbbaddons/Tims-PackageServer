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

use crate::{package::xml::PackageXML, version::Version};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use sha2::{Digest, Sha256};
use std::{ffi::OsStr, path::Path};

static PACKAGE_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r#"^([a-z0-9_-]+\.[a-z0-9_-]+(?:\.[a-z0-9_-]+)+)$"#)
        .case_insensitive(true)
        .build()
        .unwrap()
});

pub type PackageVersions = Vec<PackageInfo>;

#[derive(Debug)]
pub struct PackageList {
    pub packages: Vec<PackageVersions>,
    pub updated_at: std::time::SystemTime,
    pub updated_in: std::time::Duration,
    pub scanned_version_count: u32,
}

#[derive(Debug)]
pub struct PackageInfo {
    pub data: PackageXML,
    pub hash: String,
    pub mtime: Option<std::time::SystemTime>,
}

fn get_package_xml_from_tar<T: std::io::Read>(
    mut tar: tar::Archive<T>,
) -> crate::Result<PackageXML> {
    for file in tar.entries()? {
        let file = file?;
        let header = file.header();

        if !header.entry_type().is_file() {
            continue;
        }

        let path = header.path()?;
        let name = path
            .file_name()
            .ok_or("Path has no name")?
            .to_str()
            .ok_or("Failed to convert name to UTF-8 string")?;

        if name != "package.xml" {
            continue;
        }

        return PackageXML::try_from(file).map_err(Into::into);
    }

    Err("package.xml missing".into())
}

fn fix_string(s: &str) -> String {
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:\r\n|\r|\n)\s*|\s+").unwrap());

    REGEX.replace_all(s, " ").to_string()
}

fn read_package_archive(
    path: &Path,
    package_name: &str,
    package_version: Version,
) -> crate::Result<PackageInfo> {
    log::debug!("Reading archive {:?}", path);

    let file = std::fs::File::open(path)?;
    let mtime = file.metadata().and_then(|m| m.modified()).ok();

    let mut package_xml = get_package_xml_from_tar(tar::Archive::new(file))?;

    if package_xml.name != package_name {
        return Err(format!(
            "Package name “{}” does not match directory name “{}”",
            package_xml.name, package_name
        )
        .into());
    }

    if package_xml.package_information.version != package_version {
        return Err(format!(
            "Package version “{}” does not match filename “{}”",
            package_xml.package_information.version, package_version
        )
        .into());
    }

    for version in &mut package_xml.package_information.name {
        version.name = fix_string(&version.name);
    }

    for version in &mut package_xml.package_information.description {
        version.description = fix_string(&version.description);
    }

    package_xml.author_information.author = fix_string(&package_xml.author_information.author);

    if let Some(author_url) = package_xml.author_information.author_url.as_mut() {
        *author_url = fix_string(author_url);
    }

    let hash = {
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::open(path)?;
        let _ = std::io::copy(&mut file, &mut hasher)?;
        format!("{:x}", hasher.finalize())
    };

    Ok(PackageInfo {
        data: package_xml,
        hash,
        mtime,
    })
}

fn scan_package_dir(
    path: &Path,
    package_name: &str,
    scanned_version_count: &mut u32,
) -> crate::Result<PackageVersions> {
    log::debug!("Scanning {:?}", path);

    let mut versions = Vec::new();

    for entry in path.read_dir()? {
        *scanned_version_count += 1;

        let entry = entry?;
        let path = entry.path();

        let name = path
            .file_name()
            .ok_or("Path has no name")?
            .to_str()
            .ok_or("Failed to convert name to UTF-8 string")?;

        if name.starts_with('.') {
            log::info!("Skipping dotfile {:?}", path);
            continue;
        }

        let version_str = path
            .file_stem()
            .ok_or("Path has no file stem")?
            .to_str()
            .ok_or("Failed to convert file stem to UTF-8 string")?
            .replace('_', " ");

        match Version::parser(&version_str) {
            Ok((_, version)) => {
                if !path.is_file() {
                    log::info!("Skipping {:?}, not a file", path);
                    continue;
                }

                if path.extension() == Some(OsStr::new("txt")) {
                    // Ignore the download counter files
                    continue;
                }

                if path.extension() != Some(OsStr::new("tar")) {
                    log::info!("Skipping {:?}, not a `tar` file", path);
                    continue;
                }

                match read_package_archive(&path, package_name, version) {
                    Ok(package_info) => {
                        versions.push(package_info);
                    }
                    Err(err) => {
                        log::error!("Failed to read archive {:?}: {}", path, err);
                    }
                }
            }
            Err(err) => {
                log::error!("Skipping {:?}: Failed to parse version: {}", path, err);
            }
        }
    }

    versions.sort_by(|a, b| {
        let v1 = &a.data.package_information.version;
        let v2 = &b.data.package_information.version;

        v1.cmp(v2)
    });

    Ok(versions)
}

pub fn scan_packages() -> crate::Result<PackageList> {
    let updated_in = std::time::Instant::now();
    let mut packages = Vec::new();
    let mut scanned_version_count: u32 = 0;

    for entry in crate::SETTINGS.package_dir.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .ok_or("Path has no name")?
            .to_str()
            .ok_or("Failed to convert name to UTF-8 string")?;

        if name.starts_with('.') {
            log::info!("Skipping dotfile {:?}", path);
            continue;
        }
        if name == "auth.json" || name == "auth.json.example" {
            continue;
        }
        if !PACKAGE_ID_REGEX.is_match(name) {
            log::info!("Skipping {:?}, invalid package identifier", path);
            continue;
        }
        if !path.is_dir() {
            log::info!("Skipping {:?}, not a directory", path);
            continue;
        }

        match scan_package_dir(&path, name, &mut scanned_version_count) {
            Ok(versions) => {
                if versions.is_empty() {
                    log::warn!("No versions for package {} found", name);
                } else {
                    packages.push(versions);
                }
            }
            Err(err) => {
                log::error!("Failed to scan {:?}: {}", path, err);
            }
        }
    }

    packages.sort_by(|a, b| a[0].data.name.cmp(&b[0].data.name));

    let list = PackageList {
        packages,
        updated_at: std::time::SystemTime::now(),
        updated_in: updated_in.elapsed(),
        scanned_version_count,
    };

    log::debug!(
        "Scanned {} packages in {} seconds",
        list.scanned_version_count,
        list.updated_in.as_secs_f32()
    );

    Ok(list)
}
