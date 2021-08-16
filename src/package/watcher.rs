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
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    ffi::OsStr,
    path::Path,
    sync::{mpsc, mpsc::Receiver, Arc, Mutex},
};

pub struct PackageWatcher<'a> {
    inner: RecommendedWatcher,
    path: &'a Path,
    scanning: Arc<Mutex<()>>,
}

impl<'a> PackageWatcher<'a> {
    pub fn new(path: &'a Path) -> notify::Result<Self> {
        let (tx, rx) = mpsc::channel();

        let mut inner = notify::watcher(tx, std::time::Duration::from_secs(5))?;

        inner.watch(&path, RecursiveMode::Recursive)?;

        let mut watcher = Self {
            inner,
            path,
            scanning: Arc::new(Mutex::new(())),
        };

        watcher.start_watcher(rx);

        Ok(watcher)
    }

    fn start_scan(&self) {
        let maybe_locked = self.scanning.try_lock();

        if maybe_locked.is_err() {
            // A scan is already running
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

    fn handle_event(&self, event: DebouncedEvent) {
        match event {
            DebouncedEvent::NoticeWrite(ref path)
            | DebouncedEvent::NoticeRemove(ref path)
            | DebouncedEvent::Create(ref path)
            | DebouncedEvent::Write(ref path)
            | DebouncedEvent::Chmod(ref path)
            | DebouncedEvent::Remove(ref path)
            | DebouncedEvent::Rename(ref path, _) => {
                if path.extension() == Some(OsStr::new("tar"))
                    || path == &self.path.join("auth.json")
                {
                    log::trace!("Re-scan triggered by event: {:#?}", event);
                    self.start_scan();
                }
            }
            DebouncedEvent::Rescan => {
                log::trace!("Re-scan triggered by DebouncedEvent::Rescan");
                self.start_scan();
            }
            DebouncedEvent::Error(..) => {
                unreachable!("Error events have been handled in `start_thread` already");
            }
        }
    }

    fn start_watcher(&mut self, rx: Receiver<DebouncedEvent>) {
        loop {
            match rx.recv() {
                Ok(DebouncedEvent::Error(err, Some(path))) => {
                    log::error!("Watch error in path {:?}: {:?}", path, err);
                }
                Ok(DebouncedEvent::Error(err, None)) => {
                    log::error!("Watch error: {:?}", err);
                }
                Err(err) => {
                    log::error!("Generic watch error, stopping loop: {:?}", err);
                    break;
                }
                Ok(event) => self.handle_event(event),
            }
        }
    }
}

impl std::fmt::Debug for PackageWatcher<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackageWatcher")
            .field("path", &self.path)
            .finish()
    }
}

impl Drop for PackageWatcher<'_> {
    fn drop(&mut self) {
        self.inner.unwatch(&self.path).unwrap();
    }
}
