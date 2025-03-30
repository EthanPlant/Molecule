//! The Virtual Filesystem (VFS) provides an abstraction layer on top of a mounted filesystem.
//!
//! To manipulate files, the VFS should be used rather than calling into a filesystem directly.

use alloc::sync::Arc;

use entry::VfsEntry;
use resolver::{ResolutionSettings, Resolved};
use spin::Once;

use super::attributes::{FileType, Stat};
use super::path::Path;
use super::perm::{AccessProfile, S_ISGID};
use super::FileLocation;

pub(super) mod entry;
pub(super) mod mountpoint;
pub(super) mod node;
pub mod resolver;

/// The root of the VFS
static ROOT: Once<Arc<VfsEntry>> = Once::new();

/// Errors that can be returned from VFS operations
#[derive(Debug)]
pub enum VfsError {
    /// Attempted to write to a read-only filesystem
    ReadOnly,
    /// Insufficient permission to perform the operation
    InsufficientPermission,
    /// Parent is not a directory
    NotADirectory,
    /// Attempted to create a file that already exists
    FileAlreadyExists,
    /// Attempted to access a non-existent file
    FileDoesntExist,
    /// Path is too long
    NameTooLong,
    /// Encountered too many symbolic links
    TooManyLinks,
    /// Attempted to read from or write to an offset that is too large for the file
    OffsetTooLarge,
    /// Attempted to read or write more than [usize::MAX] bytes.
    Overflow,
    /// Attempted to read content from or write content to a directory
    IsADirectory,
    /// Attempted to perform an operation on an invalid file type
    InvalidFileType,
}

pub type VfsResult<T> = Result<T, VfsError>;

/// Return the file at a given path.
pub fn get_file_from_path(path: &Path, settings: &ResolutionSettings) -> VfsResult<Arc<VfsEntry>> {
    get_file_from_path_opt(path, settings)?.ok_or(VfsError::FileDoesntExist)
}

/// Return the file at qa given path, returning `None` if the file doesn't exist
pub fn get_file_from_path_opt(
    path: &Path,
    settings: &ResolutionSettings,
) -> VfsResult<Option<Arc<VfsEntry>>> {
    let file = match resolver::resolve_path(path, settings)? {
        Resolved::Found(file) => Some(file),
        _ => None,
    };

    Ok(file)
}

/// Create a file, add it to the VFS, then return it.
pub fn create_file(
    parent: &Arc<VfsEntry>,
    name: &str,
    ap: &AccessProfile,
    mut stat: Stat,
) -> VfsResult<Arc<VfsEntry>> {
    let parent_stat = parent.status()?;
    if parent_stat.file_type()? != FileType::Directory {
        return Err(VfsError::NotADirectory);
    }

    if !ap.can_write_dir(&parent_stat) {
        return Err(VfsError::InsufficientPermission);
    }

    stat.set_user(ap.effective_uid());
    let gid = if parent_stat.mode().has_permission(S_ISGID) {
        parent_stat.gid()
    } else {
        ap.effective_gid()
    };
    stat.set_group(gid);
    let parent_node = parent.node();
    let (id, ops) = parent_node
        .ops()
        .add_file(&parent_node.location(), name, stat)?;
    let location = FileLocation {
        mountpoint_id: parent_node.location().mountpoint_id,
        file_id: id,
    };
    let node = node::get_or_insert(location, ops);

    let entry = Arc::new(VfsEntry::new(node, name, Some(parent.clone())));
    Ok(entry)
}

/// Initialize the VFS root
pub fn init_root(root: Arc<VfsEntry>) {
    ROOT.call_once(|| root);
}

/// Get the root of the VFS entry
pub fn root() -> Arc<VfsEntry> {
    ROOT.get().expect("vfs: root not initialized").clone()
}
