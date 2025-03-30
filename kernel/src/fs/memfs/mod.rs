//! The memory filesystem (memfs) is a temporary in-memory filesystem. In tmpfs files do not
//! exist on any persistent storage and instead live fully in memory.

use alloc::boxed::Box;
use alloc::sync::Arc;

use node::{Node, NodeStorage};

use super::{FileId, FileSystem, ROOT_ID};
use crate::sync::Mutex;

mod node;

/// A temporary filesystem. This filesystem is a collection of in-memory [Node]s containing each
/// file's metadata and content.
pub struct MemFs {
    readonly: bool,
    nodes: Mutex<NodeStorage>,
}

impl MemFs {
    /// Create a new tmpfs.
    pub fn new(readonly: bool) -> Self {
        let root = Node::new();
        Self {
            readonly,
            nodes: Mutex::new(NodeStorage::new(root)),
        }
    }
}

impl FileSystem for MemFs {
    fn init(readonly: bool) -> Arc<dyn FileSystem> {
        log::debug!("Initializing memfs with readonly={}", readonly);
        Arc::new(MemFs::new(readonly))
    }

    fn get_root(&self) -> FileId {
        ROOT_ID
    }

    fn node_from_id(
        &self,
        id: super::FileId,
    ) -> Option<alloc::boxed::Box<dyn super::vfs::node::VfsNodeOps>> {
        let lock = self.nodes.lock();
        let node = lock.get_node(id)?;
        Some(Box::new(node.clone()))
    }
}
