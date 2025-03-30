use alloc::sync::Arc;
use core::str;

use cpio::CpioParser;

use super::attributes::{FileType, Mode, Stat};
use super::path::Path;
use super::perm::{AccessProfile, Gid, Uid};
use super::vfs::entry::VfsEntry;
use super::vfs::resolver::ResolutionSettings;
use super::vfs::{self, VfsError, VfsResult};

mod cpio;

/// Load the initramfs at the root of the VFS
pub fn load(initramfs: &limine::file::File) -> VfsResult<()> {
    // Safety: The initramfs data is initialized by Limine
    let data = unsafe { core::slice::from_raw_parts(initramfs.addr(), initramfs.size() as usize) };
    let cpio_parser = CpioParser::new(data);

    let mut curr_parent = (Path::root(), vfs::root());

    for entry in cpio_parser {
        let header = entry.get_header();
        let path = Path::new(entry.get_filename())?;
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
            &AccessProfile::KERNEL,
            Stat::new(
                Mode::from_u32(header.mode as _),
                0,
                Uid::new(header.uid),
                Gid::new(header.gid),
                0,
                header.rdev as _,
                header.rdev as _,
            ),
        );
        let file = match create_result {
            Ok(file) => file,
            Err(VfsError::FileAlreadyExists) => continue,
            Err(e) => return Err(e),
        };
        match file.get_type()? {
            FileType::Regular | FileType::Link => {
                let content = entry.get_content();
                file.node()
                    .ops()
                    .write_content(&file.node().location(), 0, content)?;
            }
            _ => continue,
        }
    }

    Ok(())
}

/// Update the parent used for loading the initramfs to a new path.
fn update_parent<'a>(new: &'a Path, parent: &mut (&'a Path, Arc<VfsEntry>)) -> VfsResult<()> {
    let result = match new.strip_prefix(parent.0) {
        Some(suffix) => {
            let rs = ResolutionSettings {
                cwd: Some(parent.1.clone()),
                ..ResolutionSettings::kernel_no_follow()
            };
            vfs::get_file_from_path(suffix, &rs)
        }
        None => vfs::get_file_from_path(new, &ResolutionSettings::kernel_no_follow()),
    };

    match result {
        Ok(file) => {
            *parent = (new, file);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
