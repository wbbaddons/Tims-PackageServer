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

mod macros;

use crate::SOURCE_FILES;
use fluent_templates::{
    fluent_bundle::{concurrent::FluentBundle, FluentResource},
    fs::resource_from_str,
    loader, LanguageIdentifier, Loader, StaticLoader,
};
use once_cell::sync::Lazy;
use std::{
    collections::{hash_map::Entry, HashMap},
    ffi::OsStr,
    path::PathBuf,
};

pub static DEFAULT: Lazy<LanguageIdentifier> =
    Lazy::new(|| "en-US".parse().expect("invalid fallback language"));

pub static AVAILABLE: Lazy<Vec<LanguageIdentifier>> =
    Lazy::new(|| FLUENT_LOADER.locales().cloned().collect::<Vec<_>>());

pub static FLUENT_LOADER: Lazy<StaticLoader> = Lazy::new(|| {
    #[cfg(not(test))]
    /// Do nothing
    fn customizer(_: &mut FluentBundle<&'static FluentResource>) {}

    #[cfg(test)]
    /// Inject a couple of test messages
    fn customizer(bundle: &mut FluentBundle<&'static FluentResource>) {
        static RES: Lazy<FluentResource> = Lazy::new(|| {
            resource_from_str(
                r#"
CARGO_TEST_STRING_1 = Static String
CARGO_TEST_STRING_2 = foo = { $foo }
CARGO_TEST_STRING_3 = bar = { $bar }
CARGO_TEST_STRING_4 = baz = { $baz }
CARGO_TEST_STRING_5 = foo = { $foo }
                      bar = { $bar }
                      barbaz = { $barbaz }
"#,
            )
            .unwrap()
        });

        bundle.add_resource(&RES).unwrap();
        bundle.set_use_isolating(false);
    }

    static CORE_RESOURCE: Lazy<Option<FluentResource>> = Lazy::new(|| {
        let core_ftl = SOURCE_FILES.get("assets/locales/core.ftl").unwrap();

        match resource_from_str(std::str::from_utf8(core_ftl.contents).unwrap()) {
            Ok(resource) => Some(resource),
            Err(err) => {
                log::error!(
                    "Failed to parse Fluent file assets/locales/core.ftl: {}",
                    err
                );
                None
            }
        }
    });

    static RESOURCES: Lazy<HashMap<loader::LanguageIdentifier, Vec<FluentResource>>> =
        Lazy::new(|| {
            let mut resources = HashMap::new();

            let locale_assets = SOURCE_FILES
                .into_iter()
                .filter(|entry| entry.0.starts_with("assets/locales/"));

            let mut locale_map: HashMap<LanguageIdentifier, Vec<FluentResource>> = HashMap::new();

            for (&name, source_file) in locale_assets {
                let raw_path = PathBuf::from(name);
                let path = raw_path.strip_prefix("assets/locales/").unwrap();

                if let Some(language_dir) = path.parent() {
                    if language_dir.file_stem().is_none() {
                        continue;
                    }

                    if path.extension() != Some(OsStr::new("ftl")) {
                        continue;
                    }

                    let locale_str = language_dir
                        .components()
                        .next()
                        .unwrap()
                        .as_os_str()
                        .to_str()
                        .unwrap();

                    if let Ok(locale) = locale_str.parse::<loader::LanguageIdentifier>() {
                        match resource_from_str(std::str::from_utf8(source_file.contents).unwrap())
                        {
                            Ok(resource) => match locale_map.entry(locale) {
                                Entry::Occupied(list) => {
                                    list.into_mut().push(resource);
                                }
                                Entry::Vacant(map) => {
                                    map.insert(vec![resource]);
                                }
                            },
                            Err(err) => {
                                log::error!("Failed to parse Fluent file {}: {}", name, err);
                            }
                        }
                    }
                }
            }

            for (locale, locale_resources) in locale_map {
                resources.insert(locale, locale_resources);
            }

            resources
        });

    static LOCALES: Lazy<Vec<loader::LanguageIdentifier>> =
        Lazy::new(|| RESOURCES.keys().cloned().collect());

    static FALLBACKS: Lazy<HashMap<loader::LanguageIdentifier, Vec<loader::LanguageIdentifier>>> =
        Lazy::new(|| loader::build_fallbacks(&*LOCALES));

    static BUNDLES: Lazy<
        HashMap<loader::LanguageIdentifier, FluentBundle<&'static FluentResource>>,
    > = Lazy::new(|| loader::build_bundles(&*RESOURCES, CORE_RESOURCE.as_ref(), customizer));

    StaticLoader::new(&BUNDLES, &FALLBACKS, DEFAULT.clone())
});

pub fn lookup(id: &str, lang_id: &str, json_args: &serde_json::Value) -> crate::Result<String> {
    use serde_json::Value;

    let lang = lang_id.parse()?;
    let json_args = json_args.as_object().unwrap();

    let args = {
        let map: HashMap<&str, _> = json_args
            .into_iter()
            .filter_map(|(key, value)| {
                let value = match value {
                    Value::Number(n) => n.as_f64().unwrap().into(),
                    Value::String(s) => s.clone().into(),
                    _ => return None,
                };

                Some((&**key, value))
            })
            .collect();

        if map.is_empty() {
            None
        } else {
            Some(map)
        }
    };

    let loader = &*crate::fluent::FLUENT_LOADER;
    let response = loader.lookup_complete(&lang, id, args.as_ref());

    Ok(response)
}
