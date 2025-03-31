//! VFS path resolution

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::str;

use super::entry::{EntryChild, VfsEntry};
use super::node::{self, VfsNode};
use super::{VfsError, VfsResult};
use crate::fs::attributes::FileType;
use crate::fs::path::{Component, Path};
use crate::fs::perm::AccessProfile;
use crate::fs::FileLocation;

/// Maximum amount of recursive calls to resolve a symbolic link
/// before giving up and returning an error. This is to prevent infinite loops
const SYMLOOP_MAX: usize = 8;

/// Setttings for a paath resolution operation
#[derive(Debug)]
pub struct ResolutionSettings {
    /// VFS entry of the root to begin resolution from
    pub root: Arc<VfsEntry>,
    /// VFS entry of the current working directory. If `None`, the root will be used.
    pub cwd: Option<Arc<VfsEntry>>,
    /// The access profile to use for resolution
    pub access_profile: AccessProfile,
    /// If `true`, the path is resolved for file creation, meaning the operation will not fail if
    /// the file doesn't exist.
    pub create: bool,
    /// If `true`, and the last component of the path is a symbolinc link, the link will be
    /// resolved to its target.
    pub follow_link: bool,
}

impl ResolutionSettings {
    /// Kernel access, following symbolic links
    pub fn kernel_follow() -> Self {
        Self {
            root: super::root(),
            cwd: None,
            access_profile: AccessProfile::KERNEL,
            create: false,
            follow_link: true,
        }
    }

    /// Kernel access, not following symbolic links
    pub fn kernel_no_follow() -> Self {
        Self {
            follow_link: false,
            ..Self::kernel_follow()
        }
    }
}

/// The result of a path resolution operation.
pub enum Resolved {
    /// The file was found in the VFS
    Found(Arc<VfsEntry>),
    /// The file was not found, but the parent directory was found. The file can be created.
    Createable {
        _parent: Arc<VfsEntry>,
        _name: String,
    },
}

/// Resolve  a path with the given settings.
pub fn resolve_path(path: &Path, settings: &ResolutionSettings) -> VfsResult<Resolved> {
    if settings.cwd.is_none() && path.is_empty() {
        return Err(VfsError::FileDoesntExist);
    }

    resolve_path_inner(path, settings, 0)
}

/// Resolve a VFS entry with the given name in the given lookup directory
fn resolve_entry(lookup_dir: &Arc<VfsEntry>, name: &str) -> VfsResult<Option<Arc<VfsEntry>>> {
    let mut children = lookup_dir.children();
    if let Some(ent) = children.get(name.as_bytes()) {
        return Ok(Some(ent.entry().clone()));
    }

    // The entry is not in the cache, so we need to look it up in the filesystem
    let lookup_node = lookup_dir.node();
    let Some((entry, ops)) = lookup_node
        .ops()
        .entry_by_name(&lookup_node.location(), name)?
    else {
        return Ok(None);
    };
    let node = node::insert(VfsNode::new(
        FileLocation {
            mountpoint_id: lookup_node.location().mountpoint_id,
            file_id: entry.id,
        },
        ops,
    ));
    let ent = Arc::new(VfsEntry::new(node, name, Some(lookup_dir.clone())));
    children.insert(EntryChild::new(ent.clone()));
    Ok(Some(ent))
}

/// Resolve a symbolic link to its target
fn resolve_link(
    link: &Arc<VfsEntry>,
    root: Arc<VfsEntry>,
    lookup_dir: Arc<VfsEntry>,
    access_profile: AccessProfile,
    symlink_rec: usize,
) -> VfsResult<Arc<VfsEntry>> {
    if symlink_rec + 1 > SYMLOOP_MAX {
        return Err(VfsError::TooManyLinks);
    }
    let contents = link.read_all()?;
    let link_path = Path::new(core::str::from_utf8(&contents).unwrap())?;
    let rs = ResolutionSettings {
        root,
        cwd: Some(lookup_dir),
        access_profile,
        create: false,
        follow_link: true,
    };
    let resolved = resolve_path_inner(link_path, &rs, symlink_rec + 1)?;
    let Resolved::Found(target) = resolved else {
        unreachable!()
    };
    Ok(target)
}

/// Inner implementation of path resolution operations
fn resolve_path_inner(
    path: &Path,
    settings: &ResolutionSettings,
    symlink_rec: usize,
) -> VfsResult<Resolved> {
    let mut lookup_dir = match (path.is_absolute(), &settings.cwd) {
        (false, Some(start)) => start.clone(),
        _ => settings.root.clone(),
    };

    let mut components = path.components();
    let Some(final_comp) = components.next_back() else {
        // The path has no components, so return the current directory
        return Ok(Resolved::Found(lookup_dir));
    };

    for comp in components {
        let lookup_dir_stat = lookup_dir.status()?;
        if !settings
            .access_profile
            .can_search_directory(&lookup_dir_stat)
        {
            return Err(VfsError::InsufficientPermission);
        };
        let name = match comp {
            Component::ParentDir => {
                if let Some(parent) = &lookup_dir.parent() {
                    lookup_dir = parent.clone();
                }
                continue;
            }
            Component::Normal(name) => name,
            _ => continue,
        };
        let entry = resolve_entry(&lookup_dir, str::from_utf8(name).unwrap())?
            .ok_or(VfsError::FileDoesntExist)?;
        match entry.get_type()? {
            FileType::Directory => lookup_dir = entry,
            FileType::Link => {
                lookup_dir = resolve_link(
                    &entry,
                    settings.root.clone(),
                    lookup_dir,
                    settings.access_profile,
                    symlink_rec,
                )?;
            }
            _ => return Err(VfsError::NotADirectory),
        }
    }
    let name = match final_comp {
        Component::RootDir | Component::CurDir => return Ok(Resolved::Found(lookup_dir)),
        Component::ParentDir => {
            if let Some(parent) = &lookup_dir.parent() {
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
    let Some(entry) = resolve_entry(&lookup_dir, str::from_utf8(name).unwrap())? else {
        return if settings.create {
            Ok(Resolved::Createable {
                _parent: lookup_dir,
                _name: String::from_utf8_lossy(name).to_string(),
            })
        } else {
            return Err(VfsError::FileDoesntExist);
        };
    };

    if settings.follow_link && entry.get_type()? == FileType::Link {
        Ok(Resolved::Found(resolve_link(
            &entry,
            settings.root.clone(),
            lookup_dir,
            settings.access_profile,
            symlink_rec,
        )?))
    } else {
        Ok(Resolved::Found(entry))
    }
}
