use alloc::boxed::Box;
use alloc::sync::Arc;
use core::borrow::Borrow;
use core::hash::{Hash, Hasher};

use hashbrown::HashSet;
use spin::Lazy;

use super::VfsResult;
use crate::fs::attributes::Stat;
use crate::fs::{DirEntry, FileId, FileLocation};
use crate::sync::Mutex;

/// List of nodes currently cached by the VFS
static USED_NODES: Lazy<Mutex<HashSet<NodeEntry>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// A VFS node. VFS nodes act as an entry point into a proper filesystem.
pub struct VfsNode {
    location: FileLocation,
    ops: Box<dyn VfsNodeOps>,
}

impl VfsNode {
    /// Create a new VFS node with the given location and operations.
    pub fn new(location: FileLocation, ops: Box<dyn VfsNodeOps>) -> Self {
        Self { location, ops }
    }

    /// Get the location of the node.
    pub fn location(&self) -> FileLocation {
        self.location
    }

    /// Get a reference to the node's underlying filesystem operations.
    pub fn ops(&self) -> &dyn VfsNodeOps {
        self.ops.as_ref()
    }
}

/// Filesystem node operations. A filesystem is required to implement these operations to provide a
/// hook for the VFS.
pub trait VfsNodeOps: Send + Sync {
    /// Get the status of a node
    fn get_stat(&self, loc: &FileLocation) -> VfsResult<Stat>;

    /// Add a file to the directory, returning its [FileId] and node operations.
    fn add_file(
        &self,
        parent: &FileLocation,
        name: &'static str,
        stat: Stat,
    ) -> VfsResult<(FileId, Box<dyn VfsNodeOps>)>;

    /// Read from the node starting at a given offset into the buffer `buf`, returning the number of
    /// bytes read.
    fn read_content(&self, loc: &FileLocation, off: usize, buf: &mut [u8]) -> VfsResult<usize>;

    /// Write from `buf` into a node at a given offset. Returns the number of bytes written to the
    /// file.
    fn write_content(&self, loc: &FileLocation, off: usize, buf: &[u8]) -> VfsResult<usize>;

    /// Return the directory entry with the given `name`, along with its node operations.
    ///
    /// If the entry does not exist, the function returns `None`.
    ///
    /// # Errors
    ///
    /// This function returns [VfsError::NotADirectory] if the node is not a directory.
    fn entry_by_name(
        &self,
        loc: &FileLocation,
        name: &str,
    ) -> VfsResult<Option<(DirEntry, Box<dyn VfsNodeOps>)>>;
}

/// Insert a new node into the VFS node cache and return it.
pub fn insert(node: VfsNode) -> Arc<VfsNode> {
    let mut used = USED_NODES.lock();
    let node = Arc::new(node);
    used.insert(NodeEntry(node.clone()));
    node
}

/// Look in the node cache for a node with the given location and return it. If the node is not
/// present, it is created and inserted.
pub(super) fn get_or_insert(location: FileLocation, ops: Box<dyn VfsNodeOps>) -> Arc<VfsNode> {
    let mut used_nodes = USED_NODES.lock();
    let node = used_nodes.get(&location).map(|e| e.0.clone());
    match node {
        Some(node) => node,
        None => {
            let node = Arc::new(VfsNode { location, ops });
            used_nodes.insert(NodeEntry(node.clone()));
            node
        }
    }
}

/// An entry in the VFS node cache.
struct NodeEntry(Arc<VfsNode>);

impl Borrow<FileLocation> for NodeEntry {
    fn borrow(&self) -> &FileLocation {
        &self.0.location
    }
}

impl PartialEq for NodeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.0.location.eq(&other.0.location)
    }
}

impl Eq for NodeEntry {}

impl Hash for NodeEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.location.hash(state);
    }
}
