//! Filesystem node cache

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::borrow::Borrow;
use core::fmt::Debug;
use core::hash::Hash;
use core::ops;

use hashbrown::HashSet;
use spin::Lazy;

use super::{VfsError, VfsResult};
use crate::fs::{DirEntry, FileLocation, Inode, Stat};
use crate::sync::Mutex;

/// The list of nodes currently in use.
static USED_NODES: Lazy<Mutex<HashSet<NodeEntry>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// A filesystem node
#[derive(Debug)]
pub struct Node {
    location: FileLocation,
    pub ops: Box<dyn NodeOps>,
}

impl Node {
    pub fn new(location: FileLocation, ops: Box<dyn NodeOps>) -> Self {
        Self { location, ops }
    }

    /// Get a reference to the node's location
    pub fn location(&self) -> &FileLocation {
        &self.location
    }
}

/// An entry in the node cache
struct NodeEntry(Arc<Node>);

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
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.location.hash(state)
    }
}

/// Insert a new node in cache
pub fn insert(node: Node) -> Arc<Node> {
    let mut used_nodes = USED_NODES.lock();
    let node = Arc::new(node);
    used_nodes.insert(NodeEntry(node.clone()));
    node
}

/// Looks in the node cache for the node with the given location and returns it. If the node is not
/// in the cache, it is created and inserted.
pub(super) fn get_or_insert(location: FileLocation, ops: Box<dyn NodeOps>) -> Arc<Node> {
    let mut used_nodes = USED_NODES.lock();
    let node = used_nodes.get(&location).map(|e| e.0.clone());
    match node {
        Some(node) => node,
        None => {
            let node = Arc::new(Node { location, ops });
            used_nodes.insert(NodeEntry(node.clone()));
            node
        }
    }
}

/// Filesystem node operations
pub trait NodeOps: Send + Sync + Debug {
    /// Get the status of the node
    fn get_stat(&self, loc: &FileLocation) -> VfsResult<Stat>;

    /// Add a file to the directory, returning its [Inode] and file handle.
    ///
    /// # Errors
    ///
    /// This function returns [VfsError] if an error occurs while adding the file.
    fn add_file(
        &self,
        parent: &FileLocation,
        name: &str,
        stat: Stat,
    ) -> VfsResult<(Inode, Box<dyn NodeOps>)>;

    fn entry_by_name<'a>(
        &self,
        loc: &FileLocation,
        name: &'a [u8],
    ) -> VfsResult<Option<(DirEntry, Box<dyn NodeOps>)>>;
}
