//! The Virtual Filesystem (VFS) provides an abstraction layer on top of a filesystem. To manipulate
//! files, the VFS should be used instead of calling the filesystems directly

mod node;

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::borrow::Borrow;
use core::hash::Hash;

use hashbrown::HashSet;
use node::Node;

use super::perm::{AccessProfile, S_ISGID};
use super::{FileLocation, FileType, Stat};
use crate::sync::Mutex;

/// Errors that can be returned from VFS operations
#[derive(Debug)]
pub enum VfsError {
    /// Attempted to write to a read-only filesystem
    ReadOnly,
    /// Encounted an I/O error
    IoError,
    /// Insufficient permission to perform the operation
    InsufficientPermission,
    /// Parent is not a directory
    NotADirectory,
    /// Attempted to create a file that already exists
    FileAlreadyExists,
    /// Attempted to access a non-existent file
    FileDoesntExist,
}

type VfsResult<T> = Result<T, VfsError>;

/// A VFS entry, representing a directory entry cached in memory
pub struct Entry {
    pub name: String,
    parent: Option<Arc<Entry>>,
    children: Mutex<HashSet<EntryChild>>,
    node: Option<Arc<Node>>,
}

impl Entry {
    /// Returns a reference to the underlying node.
    ///
    /// # Errors
    ///
    /// Returns [VfsError::FileDoesntExist] if the entry points to a nonexistent file.
    pub fn node(&self) -> VfsResult<&Arc<Node>> {
        self.node
            .as_ref()
            .map_or_else(|| Err(VfsError::FileDoesntExist), |val| Ok(val))
    }

    /// Get the status of the underlying node
    ///
    /// # Errors
    ///
    /// Returns [VfsError::FileDoesntExist] if the entry points to a non-existent file.
    pub fn status(&self) -> VfsResult<Stat> {
        let node = self.node()?;
        node.ops.get_stat(node.location())
    }
}

/// A child of a VFS entry.
struct EntryChild(Arc<Entry>);

impl Borrow<str> for EntryChild {
    fn borrow(&self) -> &str {
        &self.0.name
    }
}

impl PartialEq for EntryChild {
    fn eq(&self, other: &Self) -> bool {
        self.0.name.eq(&other.0.name)
    }
}

impl Eq for EntryChild {}

impl Hash for EntryChild {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.name.hash(state);
    }
}

/// Create a file, add it to the VFS, then return it.
///
/// # Errors
///
/// The following errors can be returned:
/// - [VfsError::ReadOnly] - The filesystem is read-only.
/// - [VfsError::IoError] - Encounted an I/O error.
/// - [VfsError::InsufficientPermission] - Insufficient permissions to create the file.
/// - [VfsError::NotADirectory] - `parent` is not a directory.
/// - [VfsError::FileAlreadyExists] - The file already exists.
/// - [VfsError::FileDoesntExist] - If `parent` doesn't exist.
pub fn create_file(
    parent: &Arc<Entry>,
    name: &str,
    ap: &AccessProfile,
    mut stat: Stat,
) -> VfsResult<Arc<Entry>> {
    let parent_stat = parent.status()?;
    if parent_stat.file_type() != FileType::Directory {
        return Err(VfsError::NotADirectory);
    }

    if !ap.can_write_dir(&parent_stat) {
        return Err(VfsError::InsufficientPermission);
    }

    stat.uid = ap.effective_uid();
    let gid = if parent_stat.mode.has_permission(S_ISGID) {
        parent_stat.gid
    } else {
        ap.effective_gid()
    };
    stat.gid = gid;
    let parent_node = parent.node()?;
    let (inode, ops) = parent_node
        .ops
        .add_file(parent_node.location(), name, stat)?;
    let location = FileLocation {
        mountpoint_id: parent_node.location().mountpoint_id,
        inode,
    };
    let node = node::get_or_insert(location, ops);
    let entry = Arc::new(Entry {
        name: name.to_string(),
        parent: Some(parent.clone()),
        children: Mutex::new(HashSet::new()),
        node: Some(node),
    });
    parent.children.lock().insert(EntryChild(entry.clone()));
    Err(VfsError::ReadOnly)
}
