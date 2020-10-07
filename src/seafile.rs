use fuse_mt::{
    CallbackResult, DirectoryEntry, FileAttr, FileType, FilesystemMT, RequestInfo, 
    ResultEmpty, ResultEntry, ResultOpen, ResultReaddir, ResultSlice, ResultStatfs,
    Statfs,
};
use libc::{ENOENT, EPERM};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use time::Timespec;

mod seafileapi;

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };

pub struct SeafileFS {
    api: seafileapi::SeafileAPI,
}

impl SeafileFS {
    pub fn new(server: &str, username: &str, password: &str) -> SeafileFS {
        SeafileFS {
            api: seafileapi::SeafileAPI::new(server, username, password),
        }
    }

    fn fileattr(req: RequestInfo, kind: FileType, perm: u16, size: u64, mtime: u64) -> FileAttr {
        FileAttr {
            size,
            blocks: 100u64,
            atime: Timespec {
                sec: mtime as i64,
                nsec: 0i32,
            },
            mtime: Timespec {
                sec: mtime as i64,
                nsec: 0i32,
            },
            ctime: Timespec {
                sec: mtime as i64,
                nsec: 0i32,
            },
            crtime: Timespec {
                sec: mtime as i64,
                nsec: 0,
            },
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

    fn destroy(&self, _req: RequestInfo) {
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
                trace!("Seafile libraries: {:#?}", libraries);
                libraries.sort_by(|a, b| a.name.cmp(&b.name));
                libraries.dedup_by(|a, b| a.name.eq(&b.name));
                let entry = match libraries
                    .into_iter()
                    .filter(|entry| entry.name.eq(&components[1].as_os_str().to_string_lossy()))
                    .nth(0)
                {
                    Some(e) => e,
                    _ => seafileapi::Library::default(),
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
                debug!("Seafile libraries: {:#?}", libraries);
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
                    .filter(|entry| entry.name.eq(&library_name.as_os_str().to_string_lossy()))
                    .nth(0)
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
                    .filter(|entry| entry.name.eq(&file_name.as_os_str().to_string_lossy()))
                    .nth(0)
                {
                    Some(e) => e,
                    _ => return Err(EPERM),
                };
                debug!("Found {:?} as match of {:?}", e, path);
                let (kind, perm, size, mtime) = match e.entry_type.as_str() {
                    "dir" => (FileType::Directory, 0o755 as u16, e.size, e.mtime),
                    _ => (FileType::RegularFile, 0o644 as u16, e.size, e.mtime),
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
        debug!("Seafile libraries: {:#?}", libraries);
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
                    .filter(|entry| entry.name.eq(&library_name.as_os_str().to_string_lossy()))
                    .nth(0)
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
    
    fn read(&self, _req: RequestInfo, path: &Path, _fh: u64, offset: u64, size: u32, callback: impl FnOnce(ResultSlice<'_>) -> CallbackResult) -> CallbackResult {
		debug!("read {:?} {} {}", path, offset, size);
		callback(Err(libc::ENOSYS))
	}
}
