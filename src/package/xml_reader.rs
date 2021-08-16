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

use super::xml::{
    AuthorInformation, Compatibility, ExcludedPackage, License, OptionalPackage,
    PackageDescription, PackageInformation, PackageName, PackageXML, RequiredPackage,
    UpdateInstruction,
};
use crate::version::Version;
use roxmltree::Node;
use std::{
    fmt::{Debug, Display},
    io::Read,
    str::Utf8Error,
};

use std::convert::TryFrom;
use unic_langid::LanguageIdentifierError;

type Result<T> = std::result::Result<T, PackageXmlError>;

trait AsError {
    fn as_source(&self) -> &(dyn std::error::Error + 'static);
}

impl AsError for dyn std::error::Error + 'static {
    fn as_source(&self) -> &(dyn std::error::Error + 'static) {
        self
    }
}

#[derive(Debug)]
pub enum PackageXmlError {
    XmlTree(roxmltree::Error),
    Utf8(Utf8Error),
    Io(std::io::Error),
    Nom(String), // :(
    InvalidLanguage(LanguageIdentifierError),

    MissingAttribute(String, &'static str),
    MissingElement(&'static str),
    MissingText(String),
    InvalidRoot(String),

    StdError(Box<dyn std::error::Error + 'static>),
}

impl Display for PackageXmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match *self {
            Self::XmlTree(ref err) => writeln!(f, "XML error: {}", err),
            Self::Utf8(err) => writeln!(f, "Invalid UTF-8: {}", err),
            Self::Io(ref err) => writeln!(f, "IO error: {}", err),
            Self::Nom(ref err) => writeln!(f, "Parse error: {}", err),
            Self::InvalidLanguage(ref err) => writeln!(f, "Invalid language: {}", err),

            Self::MissingAttribute(ref element, attribute) => writeln!(
                f,
                r#"Missing Attribute "{}" on element "{}""#,
                attribute, element
            ),
            Self::MissingElement(name) => writeln!(f, "Missing Element: {}", name),
            Self::MissingText(ref element) => writeln!(f, "Missing Text in element <{}>", element),
            Self::InvalidRoot(ref root) => {
                writeln!(f, "Expected a <package> node but found <{}>", root)
            }

            Self::StdError(ref err) => writeln!(f, "{}", err),
        }
    }
}

impl std::error::Error for PackageXmlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::XmlTree(ref err) => Some(err),
            Self::Utf8(ref err) => Some(err),
            Self::Io(ref err) => Some(err),
            Self::InvalidLanguage(ref err) => Some(err),

            Self::Nom(..)
            | Self::MissingAttribute(..)
            | Self::MissingElement(..)
            | Self::MissingText(..)
            | Self::InvalidRoot(..) => None,

            Self::StdError(ref boxed_err) => Some(boxed_err.as_source()),
        }
    }
}

impl From<LanguageIdentifierError> for PackageXmlError {
    fn from(err: LanguageIdentifierError) -> Self {
        PackageXmlError::InvalidLanguage(err)
    }
}

impl From<Box<dyn std::error::Error>> for PackageXmlError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        PackageXmlError::StdError(err)
    }
}

fn parse_package<'a, 'input: 'a>(node: Node<'a, 'input>) -> Result<PackageXML> {
    let mut package_xml = PackageXML {
        name: require_attribute(node, "name")?,
        ..PackageXML::default()
    };

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "packageinformation" => {
                parse_package_information(child, &mut package_xml.package_information)?;
            }
            "authorinformation" => {
                parse_author_information(child, &mut package_xml.author_information)?;
            }
            "requiredpackages" => {
                parse_required_packages(child, &mut package_xml.required_packages)?;
            }
            "optionalpackages" => {
                parse_optional_packages(child, &mut package_xml.optional_packages)?;
            }
            "excludedpackages" => {
                parse_excluded_packages(child, &mut package_xml.excluded_packages)?;
            }
            "instructions" => {
                parse_instructions(child, &mut package_xml.instructions)?;
            }
            "compatibility" => {
                parse_compatibility(child, &mut package_xml.compatibility)?;
            }
            _ => (),
        }
    }

    Ok(package_xml)
}

fn require_text<'a, 'input: 'a>(node: Node<'a, 'input>) -> Result<String> {
    match node.text() {
        Some(value) => Ok(value.to_owned()),
        None => Err(PackageXmlError::MissingText(
            node.tag_name().name().to_owned(),
        )),
    }
}

fn require_attribute<'a, 'input: 'a>(node: Node<'a, 'input>, name: &'static str) -> Result<String> {
    match node.attribute(name) {
        Some(value) => Ok(value.to_owned()),
        None => Err(PackageXmlError::MissingAttribute(
            node.tag_name().name().to_owned(),
            name,
        )),
    }
}

fn parse_package_information<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    info: &mut PackageInformation,
) -> Result<()> {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "packagename" => info.name.push(PackageName {
                name: require_text(child)?,
                language: child.attribute("language").map(str::parse).transpose()?,
            }),
            "packagedescription" => info.description.push(PackageDescription {
                description: child.text().unwrap_or_default().to_owned(),
                language: child.attribute("language").map(str::parse).transpose()?,
            }),
            "url" => info.url = child.text().and_then(|v| v.parse().ok()),
            "isapplication" => info.is_application = child.text() == Some("1"),
            "version" => match Version::try_from(require_text(child)?.as_str()) {
                Ok(version) => info.version = version,
                Err(err) => return Err(PackageXmlError::Nom(err.to_string())),
            },
            "date" => info.date = require_text(child)?,
            "license" => {
                info.license = child.text().and_then(|value| License::try_from(value).ok())
            }
            _ => (),
        }
    }

    Ok(())
}

fn parse_author_information<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    info: &mut AuthorInformation,
) -> Result<()> {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "author" => info.author = require_text(child)?,
            "authorurl" => info.author_url = child.text().map(ToOwned::to_owned),
            _ => (),
        }
    }

    Ok(())
}

fn parse_required_packages<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    packages: &mut Vec<RequiredPackage>,
) -> Result<()> {
    if !packages.is_empty() {
        log::warn!("Multiple definitions of the <requiredpackages> element found, only the last one will be used!");
    }

    // WoltLab Suite's package server handling only uses the
    // last occurence of the `<requiredpackages>` element.
    packages.clear();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        if child.tag_name().name() == "requiredpackage" {
            packages.push(RequiredPackage {
                identifier: require_text(child)?,
                min_version: require_attribute(child, "minversion")?,
            });
        }
    }

    Ok(())
}

fn parse_optional_packages<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    packages: &mut Vec<OptionalPackage>,
) -> Result<()> {
    if !packages.is_empty() {
        log::warn!("Multiple definitions of the <optionalpackages> element found, only the last one will be used!");
    }

    // WoltLab Suite's package server handling only uses the
    // last occurence of the `<optionalpackages>` element.
    packages.clear();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        if child.tag_name().name() == "optionalpackage" {
            packages.push(OptionalPackage {
                identifier: require_text(child)?,
            });
        }
    }

    Ok(())
}

fn parse_excluded_packages<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    packages: &mut Vec<ExcludedPackage>,
) -> Result<()> {
    if !packages.is_empty() {
        log::warn!("Multiple definitions of the <excludedpackages> element found, only the last one will be used!");
    }

    // WoltLab Suite's package server handling only uses the
    // last occurence of the `<excludedpackages>` element.
    packages.clear();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        if child.tag_name().name() == "excludedpackage" {
            packages.push(ExcludedPackage {
                identifier: require_text(child)?,
                version: child.attribute("version").map(ToOwned::to_owned),
            });
        }
    }

    Ok(())
}

fn parse_instructions<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    instructions: &mut Vec<UpdateInstruction>,
) -> Result<()> {
    match node.attribute("type") {
        Some("update") => instructions.push(UpdateInstruction {
            from_version: require_attribute(node, "fromversion")?,
        }),
        Some(_) => (),
        None => {
            return Err(PackageXmlError::MissingAttribute(
                "instructions".to_owned(),
                "type",
            ));
        }
    }

    Ok(())
}

fn parse_compatibility<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    compatibility: &mut Vec<Compatibility>,
) -> Result<()> {
    if !compatibility.is_empty() {
        log::warn!("Multiple definitions of the <compatibility> element found, only the last one will be used!");
    }

    // WoltLab Suite's package server handling only uses the
    // last occurence of the `<compatibility>` element.
    compatibility.clear();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        compatibility.push(Compatibility::try_from(
            require_attribute(child, "version")?.as_str(),
        )?);
    }

    Ok(())
}

impl PackageXML {
    pub fn try_from<T: Read>(mut reader: T) -> Result<PackageXML> {
        let mut buf = Vec::new();
        std::io::copy(&mut reader, &mut buf).map_err(PackageXmlError::Io)?;
        let xml = std::str::from_utf8(&buf).map_err(PackageXmlError::Utf8)?;
        let doc = roxmltree::Document::parse(xml).map_err(PackageXmlError::XmlTree)?;
        let root = doc.root_element();

        if !root.has_tag_name("package") {
            return Err(PackageXmlError::InvalidRoot(
                root.tag_name().name().to_owned(),
            ));
        }

        parse_package(root)
    }
}
