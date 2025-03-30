//! The temporary filesystem (tmpfs) is a temporary in-memory filesystem. In tmpfs files do not
//! exist on any persistent storage and instead live fully in memory.

use alloc::boxed::Box;
use alloc::sync::Arc;

use node::{Node, NodeStorage};

use super::{FileId, FileSystem, ROOT_ID};
use crate::sync::Mutex;

mod node;

/// A temporary filesystem. This filesystem is a collection of in-memory [Node]s containing each
/// file's metadata and content.
pub struct TmpFs {
    readonly: bool,
    nodes: Mutex<NodeStorage>,
}

impl TmpFs {
    /// Create a new tmpfs.
    pub fn new(readonly: bool) -> Self {
        let root = Node::new();
        Self {
            readonly,
            nodes: Mutex::new(NodeStorage::new(root)),
        }
    }
}

impl FileSystem for TmpFs {
    fn init(readonly: bool) -> Arc<dyn FileSystem> {
        log::debug!("Initializing tmpfs with readonly={}", readonly);
        Arc::new(TmpFs::new(readonly))
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
