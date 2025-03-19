//! VFS path resolution

use alloc::string::String;
use alloc::sync::Arc;
use core::str;

use hashbrown::HashSet;

use super::node::{self, Node};
use super::{root, Entry, EntryChild, VfsError, VfsResult};
use crate::fs::path::{Component, Path};
use crate::fs::perm::AccessProfile;
use crate::fs::{FileLocation, FileType};
use crate::sync::Mutex;

/// Maximum number of symbolic links that can be traversed recursively
pub const SYMLOOP_MAX: usize = 8;

/// Setting sfor a path resolution operation
pub struct ResolutionSettings {
    pub root: Arc<Entry>,
    pub cwd: Option<Arc<Entry>>,
    pub access_profile: AccessProfile,
    pub create: bool,
    pub follow: bool,
}

impl ResolutionSettings {
    pub fn kernel_nofollow() -> Self {
        Self {
            root: root(),
            cwd: None,
            access_profile: AccessProfile::KERN_PROFILE,
            create: false,
            follow: false,
        }
    }
}

/// The result of a path resolution operating
pub enum Resolved<'a> {
    /// File was found
    Found(Arc<Entry>),
    /// The file can be created
    Creatable { parent: Arc<Entry>, name: &'a [u8] },
}

/// Resolves a path with the given settings
pub fn resolve_path<'a>(path: &'a Path, settings: &ResolutionSettings) -> VfsResult<Resolved<'a>> {
    if settings.cwd.is_none() && path.is_empty() {
        return Err(super::VfsError::FileDoesntExist);
    }

    resolve_path_inner(path, settings, 0)
}

/// Resolve an entry with the given name in the given lookup dir
fn resolve_entry(lookup_dir: &Arc<Entry>, name: &[u8]) -> VfsResult<Option<Arc<Entry>>> {
    let mut children = lookup_dir.children.lock();
    if let Some(ent) = children.get(name) {
        return if ent.0.node.is_some() {
            Ok(Some(ent.0.clone()))
        } else {
            Ok(None)
        };
    }

    let Some((entry, ops)) = lookup_dir
        .node()?
        .ops
        .entry_by_name(&lookup_dir.node()?.location(), name)?
    else {
        return Ok(None);
    };
    let node = node::insert(Node::new(
        FileLocation {
            mountpoint_id: lookup_dir.node().unwrap().location().mountpoint_id,
            inode: entry.inode,
        },
        ops,
    ));
    let ent = Arc::new(Entry {
        name: String::try_from(unsafe { str::from_utf8_unchecked(name) }).unwrap(),
        parent: Some(lookup_dir.clone()),
        children: Mutex::new(HashSet::new()),
        node: Some(node),
    });
    children.insert(EntryChild(ent.clone()));
    Ok(Some(ent))
}

/// Resilves the symbolic link and returns the target
fn resolve_link(
    link: &Entry,
    root: Arc<Entry>,
    lookup_dir: Arc<Entry>,
    access_profile: &AccessProfile,
    symlink_rec: usize,
) -> VfsResult<Arc<Entry>> {
    if symlink_rec + 1 > SYMLOOP_MAX {
        return Err(VfsError::TooManyLinks);
    }
    todo!()
}

/// Inner implementation of path resolution operations
fn resolve_path_inner<'a>(
    path: &'a Path,
    settings: &ResolutionSettings,
    symlink_rec: usize,
) -> VfsResult<Resolved<'a>> {
    let mut lookup_dir = match (path.is_absolute(), &settings.cwd) {
        (false, Some(start)) => start.clone(),
        _ => settings.root.clone(),
    };
    let mut components = path.components();
    let Some(final_component) = components.next_back() else {
        return Ok(Resolved::Found(lookup_dir));
    };
    for comp in components {
        let lookup_dir_stat = lookup_dir.status()?;
        if !settings
            .access_profile
            .can_search_directory(&lookup_dir_stat)
        {
            return Err(VfsError::InsufficientPermission);
        }
        let name = match comp {
            Component::ParentDir => {
                if let Some(parent) = &lookup_dir.parent {
                    lookup_dir = parent.clone();
                }
                continue;
            }
            Component::Normal(name) => name,
            _ => continue,
        };
        let entry = resolve_entry(&lookup_dir, name)?.ok_or_else(|| VfsError::FileDoesntExist)?;
        match entry.get_type() {
            FileType::Directory => lookup_dir = entry,
            FileType::Link => {
                lookup_dir = resolve_link(
                    &entry,
                    settings.root.clone(),
                    lookup_dir,
                    &settings.access_profile,
                    symlink_rec,
                )?
            }
            _ => return Err(VfsError::NotADirectory),
        }
    }
    let name = match final_component {
        Component::RootDir | Component::CurDir => return Ok(Resolved::Found(lookup_dir)),
        Component::ParentDir => {
            if let Some(parent) = &lookup_dir.parent {
                lookup_dir = parent.clone();
            }
            return Ok(Resolved::Found(lookup_dir));
        }
        Component::Normal(name) => name,
    };

    let lookup_dir_stat = lookup_dir.status()?;
    if !settings
        .access_profile
        .can_search_directory(&lookup_dir_stat)
    {
        return Err(VfsError::InsufficientPermission);
    }
    let Some(entry) = resolve_entry(&lookup_dir, name)? else {
        return if settings.create {
            Ok(Resolved::Creatable {
                parent: lookup_dir,
                name,
            })
        } else {
            return Err(VfsError::FileDoesntExist);
        };
    };

    if settings.follow && entry.status()?.file_type() == FileType::Link {
        Ok(Resolved::Found(resolve_link(
            &entry,
            settings.root.clone(),
            lookup_dir,
            &settings.access_profile,
            symlink_rec,
        )?))
    } else {
        Ok(Resolved::Found(entry))
    }
}
