use crate::devices::{AttachedDuplexLink, DeviceType};
use crate::Computer;
use async_trait::async_trait;
use std::any::Any;
use std::io::Write;
use std::io::{IoSlice, IoSliceMut, Read};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use wasi_common::dir::{ReaddirCursor, ReaddirEntity};
use wasi_common::file::{FdFlags, FileType, Filestat, OFlags};
use wasi_common::{Error, ErrorExt};
use wasi_common::{SystemTimeSpec, WasiDir, WasiFile};

const DEV_MAJOR: u16 = 511;
const ETHERNET_MAJOR: u16 = 510;
const WIRELESS_MAJOR: u16 = 509;

fn make_device_number(major: u16, minor: u32) -> u32 {
    ((major as u32) << 20) | minor
}

pub fn decompose_device(device_no: u64) -> Option<(DeviceType, usize)> {
    let major = ((device_no >> 20) & ((1 << 12) - 1)) as u16;
    let minor = (device_no & ((1 << 20) - 1)) as u32;

    let dev_type = match major {
        ETHERNET_MAJOR => DeviceType::Ethernet,
        WIRELESS_MAJOR => DeviceType::Wireless,
        _ => return None,
    };

    Some((dev_type, minor as usize))
}

pub struct DevicesDir {
    computer: Arc<RwLock<Computer>>,
}

impl DevicesDir {
    pub fn new(computer: Arc<RwLock<Computer>>) -> DevicesDir {
        DevicesDir { computer }
    }
}

fn parse_dev(name: &str) -> (&str, Option<u8>) {
    let digit = name.chars().position(|c| c.is_ascii_digit());
    (
        &name[..digit.unwrap_or(name.len())],
        digit.and_then(|d| name[d..].parse().ok()),
    )
}

#[async_trait::async_trait]
impl WasiDir for DevicesDir {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn open_file(
        &self,
        _symlink_follow: bool,
        path: &str,
        flags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        let devs = &self.computer.read().unwrap().devices;

        if flags.intersects(OFlags::all()) {
            return Err(Error::not_supported().context("device supports no opening flags"));
        }

        if fdflags.intersects(FdFlags::DSYNC | FdFlags::SYNC | FdFlags::RSYNC) {
            return Err(Error::not_supported().context("SYNC family flags unsupported"));
        }

        let (name, idx) = parse_dev(path);

        match (name, idx) {
            ("ethernet" | "wireless", Some(idx)) => {
                let (dev_major, net) = if name == "ethernet" {
                    (ETHERNET_MAJOR, devs.ethernet_links.get(idx as usize))
                } else {
                    (WIRELESS_MAJOR, devs.wireless_links.get(idx as usize))
                };

                let open_file = OpenDuplexLinkFile {
                    link: net.ok_or_else(Error::not_found)?.clone(),
                    device_number: make_device_number(dev_major, idx as u32),
                    read,
                    write,
                };

                Ok(Box::new(open_file))
            }
            (".", None) => Ok(Box::new(OpenDevDirFile)),
            _ => Err(Error::not_found()),
        }
    }

    async fn open_dir(
        &self,
        _symlink_follow: bool,
        _path: &str,
    ) -> Result<Box<dyn WasiDir>, Error> {
        Err(Error::not_found().context("/dev/ does not have subdirectories"))
    }

    async fn create_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::perm().context("/dev/ is protected"))
    }

    async fn readdir(
        &self,
        cursor: ReaddirCursor,
    ) -> Result<Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>, Error> {
        let devs = &self.computer.read().unwrap().devices;
        // TODO prepend ., ..

        let inode_start = 1;
        let cursor_start = 0;
        let ethernet =
            devs.ethernet_links
                .clone()
                .into_iter()
                .enumerate()
                .map(move |(idx, _link)| ReaddirEntity {
                    next: ReaddirCursor::from(cursor_start + idx as u64 + 1),
                    inode: inode_start + cursor_start + idx as u64,
                    name: format!("ethernet{idx}"),
                    filetype: FileType::CharacterDevice,
                });

        let cursor_start = devs.ethernet_links.len() as u64;
        let wireless =
            devs.wireless_links
                .clone()
                .into_iter()
                .enumerate()
                .map(move |(idx, _link)| ReaddirEntity {
                    next: ReaddirCursor::from(cursor_start + idx as u64 + 1),
                    inode: inode_start + cursor_start + idx as u64,
                    name: format!("wireless{idx}"),
                    filetype: FileType::CharacterDevice,
                });

        Ok(Box::new(
            ethernet
                .chain(wireless)
                .map(Ok)
                .skip(u64::from(cursor) as usize),
        ))
    }

    async fn symlink(&self, _old_path: &str, _new_path: &str) -> Result<(), Error> {
        Err(Error::not_supported().context("/dev/ does not support symlinks"))
    }

    async fn remove_dir(&self, _path: &str) -> Result<(), Error> {
        Err(Error::perm().context("/dev/ is protected"))
    }

    async fn unlink_file(&self, _path: &str) -> Result<(), Error> {
        Err(Error::not_supported().context("/dev/ does not support symlinks"))
    }

    async fn read_link(&self, _path: &str) -> Result<PathBuf, Error> {
        Err(Error::not_supported().context("/dev/ does not support symlinks"))
    }

    async fn get_filestat(&self) -> Result<Filestat, Error> {
        Ok(Filestat {
            device_id: make_device_number(DEV_MAJOR, 0) as u64,
            inode: 1,
            filetype: FileType::Directory,
            nlink: 0,
            size: 0,
            atim: None,
            mtim: None,
            ctim: None,
        })
    }

    async fn get_path_filestat(
        &self,
        path: &str,
        follow_symlinks: bool,
    ) -> Result<Filestat, Error> {
        self.open_file(
            follow_symlinks,
            path,
            OFlags::empty(),
            true,
            false,
            FdFlags::empty(),
        )
        .await?
        .get_filestat()
        .await
    }

    async fn rename(
        &self,
        _path: &str,
        _dest_dir: &dyn WasiDir,
        _dest_path: &str,
    ) -> Result<(), Error> {
        Err(Error::perm().context("/dev/ is protected"))
    }

    async fn hard_link(
        &self,
        _path: &str,
        _target_dir: &dyn WasiDir,
        _target_path: &str,
    ) -> Result<(), Error> {
        Err(Error::not_supported().context("/dev/ does not support hard links"))
    }

    async fn set_times(
        &self,
        _path: &str,
        _atime: Option<SystemTimeSpec>,
        _mtime: Option<SystemTimeSpec>,
        _follow_symlinks: bool,
    ) -> Result<(), Error> {
        Err(Error::perm().context("/dev/ is protected"))
    }
}

// TODO begin to fail when device is removed from the world
struct OpenDuplexLinkFile {
    link: AttachedDuplexLink,
    device_number: u32,
    read: bool,
    write: bool,
}

#[async_trait]
impl WasiFile for OpenDuplexLinkFile {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(FileType::CharacterDevice)
    }
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::APPEND)
    }

    async fn set_fdflags(&mut self, flags: FdFlags) -> Result<(), Error> {
        if flags == FdFlags::APPEND {
            Ok(())
        } else {
            Err(Error::not_supported()
                .context("network link devices do not support flags other than append"))
        }
    }

    async fn get_filestat(&self) -> Result<Filestat, Error> {
        Ok(Filestat {
            device_id: self.device_number as u64,
            inode: 1,
            filetype: FileType::CharacterDevice,
            nlink: 0,
            size: 0,
            atim: None,
            mtim: None,
            ctim: None,
        })
    }

    async fn read_vectored<'a>(&self, bufs: &mut [IoSliceMut<'a>]) -> Result<u64, Error> {
        if !self.read {
            return Err(Error::badf().context("file opened as writeonly"));
        }

        Ok(self.link.read_buf().buf.read_vectored(bufs)? as u64)
    }

    async fn write_vectored<'a>(&self, bufs: &[IoSlice<'a>]) -> Result<u64, Error> {
        if !self.write {
            return Err(Error::badf().context("file opened as readonly"));
        }

        let mut buf = self.link.write_buf();
        let n = buf.buf.write_vectored(bufs)?;
        buf.on_send.notify(usize::MAX);

        Ok(n as u64)
    }

    fn num_ready_bytes(&self) -> Result<u64, Error> {
        if self.read {
            Ok(self.link.read_buf().buf.len() as u64)
        } else {
            Err(Error::badf().context("file opened as writeonly"))
        }
    }

    async fn readable(&self) -> Result<(), Error> {
        if self.read {
            Ok(())
        } else {
            Err(Error::badf().context("file opened as writeonly"))
        }
    }

    async fn writable(&self) -> Result<(), Error> {
        if self.write {
            Ok(())
        } else {
            Err(Error::badf().context("file opened as readonly"))
        }
    }
}

struct OpenDevDirFile;

#[async_trait]
impl WasiFile for OpenDevDirFile {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(FileType::Directory)
    }

    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        Ok(FdFlags::empty())
    }

    async fn set_fdflags(&mut self, _flags: FdFlags) -> Result<(), Error> {
        Err(Error::not_supported().context("/dev/ is a special directory"))
    }

    async fn readable(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn writable(&self) -> Result<(), Error> {
        Err(Error::not_supported().context("/dev/ is readonly"))
    }
}
