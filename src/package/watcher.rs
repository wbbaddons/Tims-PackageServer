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

use crate::{AUTH_DATA, PACKAGE_LIST, SETTINGS};
use notify::{
    event::{AccessKind, AccessMode, CreateKind, MetadataKind, ModifyKind, RemoveKind, RenameMode},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use notify_debouncer_full::{
    new_debouncer, DebounceEventHandler, DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

struct EventHandler {
    auth_json: PathBuf,
    scanning: Arc<Mutex<()>>,
}

impl DebounceEventHandler for EventHandler {
    fn handle_event(&mut self, result: DebounceEventResult) {
        match result {
            Ok(events) => {
                for event in events {
                    if event.need_rescan() {
                        self.start_scan(event);
                        break;
                    }

                    match event.kind {
                        // If a package file or the auth.json was created, removed or written to, start a scan.
                        EventKind::Create(CreateKind::File)
                        | EventKind::Remove(RemoveKind::File)
                        | EventKind::Access(AccessKind::Close(AccessMode::Write))
                        | EventKind::Modify(ModifyKind::Data(_)) => {
                            if self.path_matches(&event.paths[0]) {
                                self.start_scan(event);
                                break;
                            }
                        }

                        // If a folder was created or removed, assume it affected package files and start a scan.
                        EventKind::Create(CreateKind::Folder)
                        | EventKind::Remove(RemoveKind::Folder) => {
                            self.start_scan(event);
                            break;
                        }

                        // If permissions / ownership / the last modified time of a package file or folder changed, start a scan.
                        EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions))
                        | EventKind::Modify(ModifyKind::Metadata(MetadataKind::Ownership))
                        | EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)) => {
                            if self.path_matches(&event.paths[0]) || event.paths[0].is_dir() {
                                self.start_scan(event);
                                break;
                            }
                        }

                        // If a package file was (re)moved, start a scan.
                        EventKind::Remove(RemoveKind::Any)
                        | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                            if self.path_matches(&event.paths[0]) {
                                self.start_scan(event);
                                break;
                            }
                        }

                        // If a package file was created or moved, start a scan.
                        EventKind::Create(CreateKind::Any)
                        | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                            if self.path_matches(&event.paths[0]) {
                                self.start_scan(event);
                                break;
                            }
                        }

                        // In this case we know both the old and the new name,
                        // if either name matches a package file name or the auth.json path, start a scan.
                        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                            // Check the old path and the new path
                            if self.path_matches(&event.paths[0])
                                || self.path_matches(&event.paths[1])
                            {
                                self.start_scan(event);
                                break;
                            }
                        }

                        // If any other modification affected a package file, start a scan.
                        EventKind::Modify(_) => {
                            if self.path_matches(&event.paths[0]) {
                                self.start_scan(event);
                                break;
                            }
                        }

                        // All other unhandled events are ignored.
                        _ => {
                            log::trace!("Unhandled event: {:?}", event);
                            // Do nothing
                        }
                    }
                }
            }
            Err(errors) => {
                log::error!(
                    "Encountered {} errors while processing events:",
                    errors.len()
                );

                for (i, err) in errors.iter().enumerate() {
                    log::error!("Error #{i}: {err}");
                }
            }
        }
    }
}

impl EventHandler {
    fn path_matches(&self, path: &Path) -> bool {
        path.extension() == Some(OsStr::new("tar")) || path == self.auth_json
    }

    fn start_scan(&self, event: DebouncedEvent) {
        log::trace!("Re-scan triggered by event: {:#?}", event);

        let maybe_locked = self.scanning.try_lock();

        if maybe_locked.is_err() {
            log::trace!("A scan is already in progress.");
            return;
        }

        let scanning = Arc::clone(&self.scanning);

        std::thread::spawn(move || {
            let scanning = scanning.lock().unwrap();

            log::info!("Reading auth.json");
            match crate::auth::read_auth_json(SETTINGS.package_dir.join("auth.json")) {
                Ok(auth_data) => {
                    AUTH_DATA.store(Arc::new(auth_data));
                }
                Err(err) => {
                    log::error!("Failed to read `auth.json`: {}", err);
                }
            }

            log::info!("Re-scanning package directory");
            match crate::package::list_reader::scan_packages() {
                Ok(package_list) => {
                    PACKAGE_LIST.store(Some(Arc::new(package_list)));
                }
                Err(err) => {
                    log::error!("Failed to scan package directory: {}", err);
                }
            }

            // This is basically a `.unlock()` and a little nicer
            // then implicitly dropping the lock here.
            std::mem::drop(scanning);
        });
    }
}

pub struct PackageWatcher<'a> {
    // We need to keep the reference around, to prevent it from being dropped
    #[allow(dead_code)]
    debouncer: Debouncer<RecommendedWatcher, FileIdMap>,

    path: &'a Path,
}

impl<'a> PackageWatcher<'a> {
    pub fn new(path: &'a Path) -> notify::Result<Self> {
        let mut handler = EventHandler {
            auth_json: path.join("auth.json"),
            scanning: Arc::new(Mutex::new(())),
        };

        let mut debouncer = new_debouncer(
            std::time::Duration::from_secs(5),
            None,
            move |result: DebounceEventResult| handler.handle_event(result),
        )?;

        debouncer.watcher().watch(path, RecursiveMode::Recursive)?;
        debouncer.cache().add_root(path, RecursiveMode::Recursive);

        Ok(Self { debouncer, path })
    }
}

impl<'a> std::fmt::Debug for PackageWatcher<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackageWatcher")
            .field("path", &self.path)
            .finish()
    }
}
