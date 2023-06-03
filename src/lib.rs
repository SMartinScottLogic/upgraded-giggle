use bytes::Bytes;
use fuse_mt::{
    CallbackResult, DirectoryEntry, FileAttr, FileType, FilesystemMT, RequestInfo, ResultCreate,
    ResultEmpty, ResultEntry, ResultOpen, ResultReaddir, ResultSlice, ResultStatfs, ResultWrite,
    Statfs,
};
use libc::{ENOENT, ENOSYS, EPERM};
use libc::{S_IFMT, S_IFREG, S_IRGRP, S_IROTH, S_IRUSR, S_IRWXG, S_IRWXO, S_IRWXU, S_IWUSR};
use log::{debug, info, trace};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub mod seafileapi;

static TTL: Duration = Duration::from_secs(1);

pub struct SeafileFS {
    api: seafileapi::SeafileAPI,
}

impl SeafileFS {
    pub fn new(server: &OsString, username: &OsString, password: &OsString) -> SeafileFS {
        SeafileFS {
            api: seafileapi::SeafileAPI::new(
                &server.to_string_lossy(),
                &username.to_string_lossy(),
                &password.to_string_lossy(),
            ),
        }
    }

    fn fileattr(req: RequestInfo, kind: FileType, perm: u16, size: u64, mtime: u64) -> FileAttr {
        FileAttr {
            size,
            blocks: 100u64,
            atime: SystemTime::UNIX_EPOCH + Duration::from_secs(mtime),
            mtime: SystemTime::UNIX_EPOCH + Duration::from_secs(mtime),
            ctime: SystemTime::UNIX_EPOCH + Duration::from_secs(mtime),
            crtime: SystemTime::UNIX_EPOCH,
            kind,
            perm,
            nlink: 0u32,
            uid: req.uid,
            gid: req.gid,
            rdev: 0u32,
            flags: 0,
        }
    }
}

impl FilesystemMT for SeafileFS {
    fn init(&self, _req: RequestInfo) -> ResultEmpty {
        info!("init");
        Ok(())
    }

    fn destroy(&self) {
        info!("destroy");
    }

    fn getattr(&self, req: RequestInfo, path: &Path, _fh: Option<u64>) -> ResultEntry {
        debug!("getattr: {:?}", path);
        let mut components = path.components().collect::<Vec<_>>();
        debug!("getattr: {:?}", components);

        match components.len() {
            2 => {
                let mut libraries = match self.api.get_libraries() {
                    Ok(l) => l,
                    Err(e) => {
                        debug!("ERROR: readdir({:?}) {}", path, e);
                        return Err(ENOENT);
                    }
                };
                trace!("Seafile libraries: {:?}", libraries);
                libraries.sort_by(|a, b| a.name.cmp(&b.name));
                libraries.dedup_by(|a, b| a.name.eq(&b.name));
                let entry = match libraries
                    .into_iter()
                    .find(|entry| entry.name.eq(&components[1].as_os_str().to_string_lossy()))
                {
                    Some(e) => e,
                    _ => {
                        debug!("ERROR: 0: readdir({:?})", path);
                        return Err(ENOENT);
                    }
                };

                Ok((
                    TTL,
                    SeafileFS::fileattr(req, FileType::Directory, 0o755, entry.size, entry.mtime),
                ))
            }
            1 => Ok((
                TTL,
                SeafileFS::fileattr(req, FileType::Directory, 0o755, 0, 0),
            )),
            _ => {
                let mut libraries = match self.api.get_libraries() {
                    Ok(l) => l,
                    Err(e) => {
                        debug!("ERROR: A: readdir({:?}) {}", path, e);
                        return Err(ENOENT);
                    }
                };
                libraries.sort_by(|a, b| a.name.cmp(&b.name));
                libraries.dedup_by(|a, b| a.name.eq(&b.name));
                debug!("Seafile libraries: {:?}", libraries);
                let library_name = components.remove(1);
                let file_name = components.pop().unwrap();
                debug!(
                    "split: ({:?} | {:?} | {:?})",
                    library_name, components, file_name
                );
                let relative_path = components.into_iter().collect::<PathBuf>();
                debug!("join: {:?}", relative_path);
                let entry = match libraries
                    .into_iter()
                    .find(|entry| entry.name.eq(&library_name.as_os_str().to_string_lossy()))
                {
                    Some(e) => e,
                    _ => seafileapi::Library::default(),
                };
                let entries = match self.api.get_library_content(&entry.id, &relative_path) {
                    Ok(e) => e,
                    Err(e) => {
                        debug!("ERROR: B: readdir({:?}) {}", path, e);
                        return Err(ENOENT);
                    }
                };
                debug!(
                    "TODO: Find {:?} in entries of {}: {:?}",
                    path, entry.name, entries
                );
                let e = match entries
                    .into_iter()
                    .find(|entry| entry.name.eq(&file_name.as_os_str().to_string_lossy()))
                {
                    Some(e) => e,
                    _ => return Err(ENOENT),
                };
                debug!("Found {:?} as match of {:?}", e, path);
                let (kind, perm, size, mtime) = match e.entry_type.as_str() {
                    "dir" => (FileType::Directory, 0o755_u16, e.size, e.mtime),
                    _ => (FileType::RegularFile, 0o644_u16, e.size, e.mtime),
                };
                Ok((TTL, SeafileFS::fileattr(req, kind, perm, size, mtime)))
            }
        }
    }

    fn statfs(&self, _req: RequestInfo, path: &Path) -> ResultStatfs {
        debug!("statfs: {:?}", path);

        Ok(Statfs {
            blocks: 100u64,
            bfree: 100u64,
            bavail: 0u64,
            files: 100u64,
            ffree: 100u64,
            bsize: 100u32,
            namelen: 255u32,
            frsize: 100u32,
        })
    }

    fn mkdir(&self, req: RequestInfo, parent: &Path, name: &OsStr, _mode: u32) -> ResultEntry {
        debug!("mkdir: {:?} in {:?} {:?}", name, parent, parent.parent());
        if parent.parent().is_none() {
            return Err(EPERM);
        }
        let mut libraries = match self.api.get_libraries() {
            Ok(l) => l,
            Err(e) => {
                debug!("ERROR: mkdir({:?} {:?}) {}", parent, name, e);
                return Err(ENOENT);
            }
        };
        debug!("Seafile libraries: {:?}", libraries);
        libraries.sort_by(|a, b| a.name.cmp(&b.name));
        libraries.dedup_by(|a, b| a.name.eq(&b.name));

        let mut components = parent.components().collect::<Vec<_>>();
        let library_name = components.remove(1);
        debug!("split: ({:?}, {:?})", library_name, components);
        let mut relative_path = components.into_iter().collect::<PathBuf>();
        relative_path.push(name);
        debug!("join: {:?}", relative_path);
        let library = match libraries
            .into_iter()
            .find(|entry| entry.name.eq(&library_name.as_os_str().to_string_lossy()))
        {
            Some(e) => e,
            _ => {
                debug!("ERROR: no library {:?}", library_name);
                return Err(ENOENT);
            }
        };
        let result = match self
            .api
            .create_new_directory(&library.id, relative_path.as_path())
        {
            Ok(e) => e,
            Err(e) => {
                debug!("ERROR: mkdir({:?} {:?}) {}", parent, name, e);
                return Err(ENOENT);
            }
        };

        debug!(
            "TODO create {:?} in {:?}: {:?}",
            relative_path, library_name, result
        );

        if result == "\"success\"" {
            return Ok((
                TTL,
                SeafileFS::fileattr(req, FileType::Directory, 0o755, 0, 0),
            ));
        }

        Err(ENOSYS)
    }
    fn rmdir(&self, _req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        debug!("rmdir: {:?} in {:?} {:?}", name, parent, parent.parent());

        if parent.parent().is_none() {
            return Err(EPERM);
        }
        let mut libraries = match self.api.get_libraries() {
            Ok(l) => l,
            Err(e) => {
                debug!("ERROR: mkdir({:?} {:?}) {}", parent, name, e);
                return Err(ENOENT);
            }
        };
        debug!("Seafile libraries: {:?}", libraries);
        libraries.sort_by(|a, b| a.name.cmp(&b.name));
        libraries.dedup_by(|a, b| a.name.eq(&b.name));

        let mut components = parent.components().collect::<Vec<_>>();
        let library_name = components.remove(1);
        debug!("split: ({:?}, {:?})", library_name, components);
        let mut relative_path = components.into_iter().collect::<PathBuf>();
        relative_path.push(name);
        debug!("join: {:?}", relative_path);

        let _library = match libraries
            .into_iter()
            .find(|entry| entry.name.eq(&library_name.as_os_str().to_string_lossy()))
        {
            Some(e) => e,
            _ => {
                debug!("ERROR: no library {:?}", library_name);
                return Err(ENOENT);
            }
        };
        Err(ENOSYS)
        // DO NOT PROCEED WITH THIS -- NEED TO TEST WHETHER EMPTY FIRST - SEAFILE WILL *WIPE* CONTENTS
        /*
                let result = match self.api.delete_directory(&library.id, relative_path.as_path()) {
                    Ok(e) => e,
                    Err(e) => {
                        debug!("ERROR: rmdir({:?} {:?}) {}", parent, name, e);
                        return Err(ENOENT);
                    }
                };

                debug!("TODO remove {:?} from {:?}: {:?}", relative_path, library_name, result);

                if result == "\"success\"" {
                    return Ok(());
                }

        Err(ENOSYS)
        */
    }

    fn opendir(&self, _req: RequestInfo, path: &Path, _flags: u32) -> ResultOpen {
        debug!("opendir: {:?} (flags = {:#o})", path, _flags);
        match path.parent() {
            None => Ok((0, 0)),
            Some(_) => Ok((0, 0)), //Err(EPERM)
        }
    }

    fn readdir(&self, _req: RequestInfo, path: &Path, _fh: u64) -> ResultReaddir {
        debug!("readdir: {:?}", path);

        let mut libraries = match self.api.get_libraries() {
            Ok(l) => l,
            Err(e) => {
                debug!("ERROR: readdir({:?}) {}", path, e);
                return Err(ENOENT);
            }
        };
        debug!("Seafile libraries: {:?}", libraries);
        libraries.sort_by(|a, b| a.name.cmp(&b.name));
        libraries.dedup_by(|a, b| a.name.eq(&b.name));

        let entries = match path.parent() {
            None => libraries
                .into_iter()
                .map(|entry| DirectoryEntry {
                    name: OsString::from(entry.name),
                    kind: FileType::Directory,
                })
                .collect(),
            Some(_) => {
                let mut components = path.components().collect::<Vec<_>>();
                let library_name = components.remove(1);
                debug!("split: ({:?}, {:?})", library_name, components);
                let relative_path = components.into_iter().collect::<PathBuf>();
                debug!("join: {:?}", relative_path);
                let library = match libraries
                    .into_iter()
                    .find(|entry| entry.name.eq(&library_name.as_os_str().to_string_lossy()))
                {
                    Some(e) => e,
                    _ => {
                        debug!("ERROR: no library {:?}", library_name);
                        return Err(ENOENT);
                    }
                };
                let entries = match self
                    .api
                    .get_library_content(&library.id, relative_path.as_path())
                {
                    Ok(e) => e,
                    Err(e) => {
                        debug!("ERROR: readdir({:?}) {}", path, e);
                        return Err(ENOENT);
                    }
                };
                entries
                    .into_iter()
                    .map(|entry| DirectoryEntry {
                        name: OsString::from(entry.name),
                        kind: match entry.entry_type.as_str() {
                            "dir" => FileType::Directory,
                            _ => FileType::RegularFile,
                        },
                    })
                    .collect()
            }
        };
        debug!("readdir {:?}: {:?}", path, entries);

        Ok(entries)
    }

    fn truncate(&self, _req: RequestInfo, path: &Path, _fh: Option<u64>, size: u64) -> ResultEmpty {
        debug!("truncate: {:?} to {:#x}", path, size);
        Ok(())
    }

    fn open(&self, _req: RequestInfo, path: &Path, flags: u32) -> ResultOpen {
        debug!("open {:?} {:#o}", path, flags);
        Err(libc::ENOSYS)
    }

    fn flush(&self, _req: RequestInfo, path: &Path, fh: u64, _lock_owner: u64) -> ResultEmpty {
        debug!("flush: {:?} {}", path, fh);
        Ok(())
    }

    fn release(
        &self,
        _req: RequestInfo,
        path: &Path,
        fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> ResultEmpty {
        debug!("release: {:?} {}", path, fh);
        Ok(())
    }

    fn fsync(&self, _req: RequestInfo, path: &Path, fh: u64, _datasync: bool) -> ResultEmpty {
        debug!("fsync: {:?} {}", path, fh);
        Ok(())
    }

    fn read(
        &self,
        _req: RequestInfo,
        path: &Path,
        _fh: u64,
        offset: u64,
        size: u32,
        callback: impl FnOnce(ResultSlice<'_>) -> CallbackResult,
    ) -> CallbackResult {
        debug!("read {:?} {} {}", path, offset, size);
        let mut components = path.components().collect::<Vec<_>>();
        debug!("read: {:?}", components);

        let mut libraries = match self.api.get_libraries() {
            Ok(l) => l,
            Err(e) => {
                debug!("ERROR: A: read({:?}) {}", path, e);
                return callback(Err(ENOENT));
            }
        };
        libraries.sort_by(|a, b| a.name.cmp(&b.name));
        libraries.dedup_by(|a, b| a.name.eq(&b.name));
        debug!("Seafile libraries: {:?}", libraries);
        let library_name = components.remove(1);
        debug!("split: ({:?} | {:?})", library_name, components);
        let relative_path = components.into_iter().collect::<PathBuf>();
        debug!("join: {:?}", relative_path);
        let entry = match libraries
            .into_iter()
            .find(|entry| entry.name.eq(&library_name.as_os_str().to_string_lossy()))
        {
            Some(e) => e,
            _ => seafileapi::Library::default(),
        };
        let download_uri = match self.api.get_download_link(&entry.id, &relative_path) {
            Ok(e) => e,
            Err(e) => {
                debug!("ERROR: B: read({:?}) {}", path, e);
                return callback(Err(ENOENT));
            }
        };
        debug!(
            "TODO: Find {:?} in entries of {}: {:?}",
            path, entry.name, download_uri
        );
        let mut body = match self.api.download(&download_uri) {
            Ok(e) => e,
            Err(e) => {
                debug!("ERROR: C: read({:?}) {}", path, e);
                return callback(Err(ENOENT));
            }
        };
        if body.len() > offset as usize {
            body = body.split_off(offset as usize);
        } else {
            debug!("body len: {} emptying", body.len());
            body = Bytes::new();
        }
        if body.len() > size as usize {
            body = body.split_to(size as usize);
        }
        debug!("body: {:?}", body);

        callback(Ok(&body))
    }

    fn write(
        &self,
        _req: RequestInfo,
        path: &Path,
        _fh: u64,
        offset: u64,
        data: Vec<u8>,
        flags: u32,
    ) -> ResultWrite {
        debug!("write {:?} {} {} {:#o}", path, offset, data.len(), flags);
        Ok(data.len() as u32)
    }

    fn mknod(
        &self,
        req: RequestInfo,
        parent: &Path,
        name: &OsStr,
        mode: u32,
        rdev: u32,
    ) -> ResultEntry {
        debug!(
            "mknod: {:?}/{:?} (mode={:#o}, rdev={})",
            parent, name, mode, rdev
        );
        // Cannot create non-regular file
        if mode & S_IFMT != S_IFREG {
            return Err(EPERM);
        }
        // Only support creating with permissions of 644
        if mode & S_IRWXU != S_IRUSR | S_IWUSR {
            return Err(EPERM);
        }
        if mode & S_IRWXG != S_IRGRP {
            return Err(EPERM);
        }
        if mode & S_IRWXO != S_IROTH {
            return Err(EPERM);
        }
        // Can only create within a library
        if parent.parent().is_none() {
            return Err(EPERM);
        }

        let mut libraries = match self.api.get_libraries() {
            Ok(l) => l,
            Err(e) => {
                debug!("ERROR: mkdir({:?} {:?}) {}", parent, name, e);
                return Err(ENOENT);
            }
        };
        debug!("Seafile libraries: {:?}", libraries);
        libraries.sort_by(|a, b| a.name.cmp(&b.name));
        libraries.dedup_by(|a, b| a.name.eq(&b.name));

        let mut components = parent.components().collect::<Vec<_>>();
        let library_name = components.remove(1);
        debug!("split: ({:?}, {:?})", library_name, components);
        let mut relative_path = components.into_iter().collect::<PathBuf>();
        relative_path.push(name);
        debug!("join: {:?}", relative_path);

        let library = match libraries
            .into_iter()
            .find(|entry| entry.name.eq(&library_name.as_os_str().to_string_lossy()))
        {
            Some(e) => e,
            _ => {
                debug!("ERROR: no library {:?}", library_name);
                return Err(ENOENT);
            }
        };
        let result = match self.api.create_file(&library.id, relative_path.as_path()) {
            Ok(e) => e,
            Err(e) => {
                debug!("ERROR: mknod({:?} {:?}) {}", parent, name, e);
                return Err(ENOENT);
            }
        };

        debug!(
            "TODO create {:?} in {:?}: {:?}",
            relative_path, library_name, result
        );

        if result == "\"success\"" {
            return Ok((
                TTL,
                SeafileFS::fileattr(req, FileType::RegularFile, 0o644, 0, 0),
            ));
        }
        Err(EPERM)
    }

    fn create(
        &self,
        _req: RequestInfo,
        parent: &Path,
        name: &OsStr,
        mode: u32,
        flags: u32,
    ) -> ResultCreate {
        debug!("create {:?} {:?} {:?} {:?}", parent, name, mode, flags);
        Err(ENOSYS)
    }

    fn unlink(&self, _req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        debug!("unlink {:?} {:?}", parent, name);
        Err(ENOSYS)
    }
}
