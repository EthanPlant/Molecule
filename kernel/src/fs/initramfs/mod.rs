//! Support for the initramfs

use alloc::sync::Arc;
use core::str;

use cpio::CpioParser;

use super::path::Path;
use super::perm::{AccessProfile, Gid, Uid};
use super::vfs::resolver::ResolutionSettings;
use super::vfs::{self, VfsError, VfsResult};
use super::{Mode, Stat};

mod cpio;

/// Load the initramfs at the root of the VFS
pub fn load(initramfs: &limine::file::File) -> VfsResult<()> {
    // Safety: The initramfs data is initialized by Limine
    let data = unsafe { core::slice::from_raw_parts(initramfs.addr(), initramfs.size() as usize) };

    let mut curr_parent = (Path::root(), vfs::root());
    let cpio_parser = CpioParser::new(data);
    for entry in cpio_parser {
        let header = entry.get_header();
        let path = Path::new(entry.get_filename())?;
        log::debug!("{:?}", path);
        let Some(name) = path.file_name() else {
            continue;
        };
        let parent_path = match path.parent() {
            Some(p) if p.is_empty() => Path::root(),
            None => Path::root(),
            Some(p) => p,
        };
        update_parent(parent_path, &mut curr_parent)?;
        let create_result = vfs::create_file(
            &curr_parent.1.clone(),
            str::from_utf8(name).unwrap(),
            &AccessProfile::KERN_PROFILE,
            Stat {
                mode: Mode(header.mode as _),
                links: 0,
                uid: Uid::new(header.uid),
                gid: Gid::new(header.gid),
                size: 0,
                blocks: 0,
                dev_major: header.rdev as _,
                dev_minor: header.rdev as _,
            },
        );
        let file = match create_result {
            Ok(file_mutex) => file_mutex,
            Err(VfsError::FileAlreadyExists) => continue,
            Err(e) => return Err(e),
        };
    }
    Ok(())
}

fn update_parent<'a>(new: &'a Path, parent: &mut (&'a Path, Arc<vfs::Entry>)) -> VfsResult<()> {
    let result = match new.strip_prefix(parent.0) {
        Some(suffix) => {
            let rs = ResolutionSettings {
                cwd: Some(parent.1.clone()),
                ..ResolutionSettings::kernel_nofollow()
            };
            vfs::get_file_from_path(suffix, &rs)
        }
        None => vfs::get_file_from_path(new, &ResolutionSettings::kernel_nofollow()),
    };
    match result {
        Ok(file) => {
            *parent = (new, file);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
