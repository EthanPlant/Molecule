use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::hash::{Hash, Hasher};

use hashbrown::HashSet;

use super::node::VfsNode;
use super::VfsResult;
use crate::fs::attributes::{FileType, Stat};
use crate::fs::vfs::VfsError;
use crate::sync::{Mutex, MutexGuard};

/// A VFS entry, representing a directory entry cached in memory
pub struct VfsEntry {
    name: String,
    parent: Option<Arc<VfsEntry>>,
    children: VfsEntryChildren,
    node: Arc<VfsNode>,
}

impl VfsEntry {
    /// Create a new VFS entry for a given [VfsNode].
    pub fn new(node: Arc<VfsNode>, name: &str, parent: Option<Arc<VfsEntry>>) -> Self {
        Self {
            name: name.to_string(),
            parent,
            children: Default::default(),
            node,
        }
    }

    /// Returns a reference to the underlying node.
    pub fn node(&self) -> Arc<VfsNode> {
        self.node.clone()
    }

    /// Return a reference to the entry's parent.
    pub fn parent(&self) -> Option<Arc<VfsEntry>> {
        self.parent.clone()
    }

    /// Return a lock to the entry's children.
    pub fn children(&self) -> MutexGuard<HashSet<EntryChild>> {
        self.children.0.lock()
    }

    /// Get the status of the underlying nopde
    pub fn status(&self) -> VfsResult<Stat> {
        self.node().ops().get_stat(&self.node().location())
    }

    /// Get the filetype of the entry
    pub fn get_type(&self) -> VfsResult<FileType> {
        self.status()?.file_type()
    }

    /// Read the entire content of a file into a buffer.
    ///
    /// # Errors
    ///
    /// - Returns [VfsError::Overflow] if the file is larger than [usize::MAX] bytes.
    pub fn read_all(&self) -> VfsResult<Vec<u8>> {
        const INCREMENT: usize = 512;
        let len: usize = self.node().ops().get_stat(&self.node().location())?.size();
        let len = len.checked_add(INCREMENT).ok_or(VfsError::Overflow)?;
        let mut buf = vec![0u8; len];
        let mut off = 0;
        loop {
            if off >= buf.len() {
                let new_size = buf.len().checked_add(INCREMENT).ok_or(VfsError::Overflow)?;
                buf.resize(new_size, 0);
            }
            let len =
                self.node()
                    .ops()
                    .read_content(&self.node().location(), off, &mut buf[off..])?;
            if len == 0 {
                break;
            }
            off += len;
        }
        buf.truncate(off);
        Ok(buf)
    }
}

/// A child of a VFS entry.
///
/// An EntryChild is a smart pointer around a VFS entry, with the [Borrow], [PartialEq], and [Hash]
/// traits forwarded to the entry's name.
pub struct EntryChild(Arc<VfsEntry>);

impl EntryChild {
    /// Create a new EntryChild from a VFS entry
    pub fn new(entry: Arc<VfsEntry>) -> Self {
        Self(entry)
    }

    /// Get the underlying entry
    pub fn entry(&self) -> Arc<VfsEntry> {
        self.0.clone()
    }
}

impl Borrow<[u8]> for EntryChild {
    fn borrow(&self) -> &[u8] {
        self.0.name.as_bytes()
    }
}

impl PartialEq for EntryChild {
    fn eq(&self, other: &Self) -> bool {
        self.0.name.eq(&other.0.name)
    }
}

impl Eq for EntryChild {}

impl Hash for EntryChild {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.name.hash(state);
    }
}

/// A collection of children for a VFS entry
struct VfsEntryChildren(Mutex<HashSet<EntryChild>>);

impl VfsEntryChildren {
    /// Create a new empty list of VFS children
    pub fn new() -> Self {
        Self(Mutex::new(HashSet::new()))
    }
}

impl Default for VfsEntryChildren {
    fn default() -> Self {
        Self::new()
    }
}
