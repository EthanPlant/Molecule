use alloc::boxed::Box;
use alloc::sync::Arc;
use core::any::Any;

use attributes::FileType;
use vfs::mountpoint::{self, MountPoint};
use vfs::node::VfsNodeOps;

pub(super) mod attributes;
pub mod initramfs;
pub mod path;
pub(super) mod perm;
mod memfs;
pub mod vfs;

/// The file id of the root of a filesystem
const ROOT_ID: FileId = FileId(1);

/// Filesystem trait, marking a type as a filesystem
pub trait FileSystem: Any + Send + Sync {
    /// Load and initialize the filesystem
    fn init(readonly: bool) -> Arc<dyn FileSystem>
    where
        Self: Sized;
    /// Get the root of the filesystem
    fn get_root(&self) -> FileId;
    /// Get the VFS node of a file from its file id
    fn node_from_id(&self, id: FileId) -> Option<Box<dyn VfsNodeOps>>;
}

/// A filesystem node id. This is a unique number that represents a node in a filesystem.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u64);

impl FileId {
    /// Create a new file id
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// The location of a file, containing the file's mountpoint id and file id. With these two values
/// it is possible to uniquely identify any given file.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileLocation {
    mountpoint_id: u32,
    file_id: FileId,
}

impl FileLocation {
    /// Get the mountpoint of this file
    pub fn get_mountpoint(&self) -> Option<Arc<MountPoint>> {
        mountpoint::from_id(self.mountpoint_id)
    }

    /// Get the filesystem of the file at this location
    pub fn get_filesystem(&self) -> Option<Arc<dyn FileSystem>> {
        self.get_mountpoint().map(|mp| mp.filesystem())
    }
}

/// An entry in a directory
#[derive(Clone)]
pub struct DirEntry<'a> {
    id: FileId,
    _entry_type: FileType,
    name: &'a str,
}

/// Initializze the filesystem
pub fn init() {
    log::debug!("fs: Initializing filesystem");
    let root = mountpoint::create_root();
    vfs::init_root(root);
    log::debug!("fs: Filesystem initialized");
}
