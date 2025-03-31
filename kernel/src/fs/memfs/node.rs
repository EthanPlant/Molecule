use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::any::Any;
use core::cmp::{max, min};

use super::MemFs;
use crate::fs::attributes::{FileType, Mode, Stat};
use crate::fs::devfs::{DeviceId, DeviceType};
use crate::fs::perm::{
    Gid, Uid, ROOT_GID, ROOT_UID, S_IRGRP, S_IROTH, S_IRUSR, S_ISGID, S_IWGRP, S_IWOTH, S_IWUSR,
    S_IXGRP, S_IXOTH, S_IXUSR,
};
use crate::fs::vfs::node::VfsNodeOps;
use crate::fs::vfs::{VfsError, VfsResult};
use crate::fs::{devfs, DirEntry, FileId, FileLocation, ROOT_ID};
use crate::sync::Mutex;

/// Cache of memfs nodes
pub struct NodeStorage(Vec<Option<Node>>);

impl NodeStorage {
    /// Create a new node storage with a given root node
    pub fn new(root: Node) -> Self {
        Self(vec![Some(root)])
    }

    /// Returns a free slot for a new node
    pub fn get_free(&mut self) -> (FileId, &mut Option<Node>) {
        let slot = self
            .0
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.is_none())
            .map(|(i, _)| i);
        let index = match slot {
            Some(i) => i,
            None => {
                let i = self.0.len();
                self.0.push(None);
                i
            }
        };
        let id = FileId::new(index as u64 + 1);
        let slot = &mut self.0[index];
        (id, slot)
    }

    /// Get the node with a given id
    pub fn get_node(&self, id: FileId) -> Option<&Node> {
        let index = (id.0 as usize).checked_sub(1)?;
        self.0.get(index).and_then(Option::as_ref)
    }
}

/// A node in the tmpfs, representing a file.
#[derive(Clone, Debug)]
pub struct Node(Arc<Mutex<NodeInner>>);

impl Node {
    /// Create a new empty node. This creates an empty directory node with all permissions owned by
    /// the root user and group.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(NodeInner {
            mode: Mode::new(
                FileType::Directory,
                S_ISGID
                    | S_IRGRP
                    | S_IROTH
                    | S_IRUSR
                    | S_IWGRP
                    | S_IWOTH
                    | S_IWUSR
                    | S_IXGRP
                    | S_IXOTH
                    | S_IXUSR,
            ),
            link: 2,
            uid: ROOT_UID,
            gid: ROOT_GID,
            content: NodeContent::Directory(vec![DirEntry {
                id: ROOT_ID,
                _entry_type: FileType::Directory,
                name: ".".to_string(),
            }]),
        })))
    }

    /// Create a new node from the given status
    ///
    /// # Errors
    ///
    /// Returns [VfsError::InvalidFileType] if the status's file type is invalid.
    pub fn new_from_stat(
        stat: &Stat,
        id: Option<FileId>,
        parent: Option<FileId>,
    ) -> VfsResult<Self> {
        let file_type = stat.file_type()?;
        let content = match file_type {
            FileType::Regular => NodeContent::Regular(Vec::new()),
            FileType::Directory => {
                let mut entries = Vec::new();
                if let Some(id) = id {
                    entries.push(DirEntry {
                        id,
                        _entry_type: FileType::Directory,
                        name: ".".to_string(),
                    });
                };
                if let Some(parent) = parent {
                    entries.push(DirEntry {
                        id: parent,
                        _entry_type: FileType::Directory,
                        name: "..".to_string(),
                    });
                };
                NodeContent::Directory(entries)
            }
            FileType::Link => NodeContent::Link(Vec::new()),
            FileType::Fifo => NodeContent::Fifo,
            FileType::Socket => NodeContent::Socket,
            FileType::BlockDevice => NodeContent::BlockDevice {
                major: stat.dev_major(),
                minor: stat.dev_minor(),
            },
            FileType::CharDevice => NodeContent::CharDevice {
                major: stat.dev_major(),
                minor: stat.dev_minor(),
            },
        };
        let link = if file_type == FileType::Directory {
            2
        } else {
            1
        };

        Ok(Self(Arc::new(Mutex::new(NodeInner {
            mode: stat.mode(),
            link,
            uid: stat.uid(),
            gid: stat.gid(),
            content,
        }))))
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}

impl VfsNodeOps for Node {
    fn get_stat(&self, _loc: &FileLocation) -> VfsResult<Stat> {
        let inner = self.0.lock();
        Ok(inner.as_stat())
    }

    fn add_file(
        &self,
        parent: &FileLocation,
        name: &str,
        stat: Stat,
    ) -> VfsResult<(FileId, Box<dyn VfsNodeOps>)> {
        let fs = parent.get_filesystem().expect("tmpfs is mounted");
        let fs = fs as Arc<dyn Any>;
        let fs = fs.downcast_ref::<MemFs>().expect("parent is in tmpfs");
        if fs.readonly {
            return Err(VfsError::ReadOnly);
        }

        let entry_type = stat.file_type()?;
        let mut nodes = fs.nodes.lock();
        let (id, slot) = nodes.get_free();

        let mut parent_inner = self.0.lock();
        let NodeContent::Directory(parent_entries) = &mut parent_inner.content else {
            return Err(VfsError::NotADirectory);
        };
        let node = Node::new_from_stat(&stat, Some(id), Some(parent.file_id))?;
        let ent = DirEntry {
            id,
            _entry_type: entry_type,
            name: name.to_string(),
        };
        let res = parent_entries.binary_search_by(|ent| ent.name.cmp(&name.to_string()));
        let Err(ent_index) = res else {
            return Err(VfsError::FileAlreadyExists);
        };
        parent_entries.insert(ent_index, ent);
        *slot = Some(node.clone());
        if entry_type == FileType::Directory {
            parent_inner.link += 1;
        }
        Ok((id, Box::new(node)))
    }

    fn read_content(&self, _loc: &FileLocation, off: usize, buf: &mut [u8]) -> VfsResult<usize> {
        let inner = self.0.lock();
        let content = match &inner.content {
            NodeContent::Regular(content) | NodeContent::Link(content) => content,
            NodeContent::Directory(_) => return Err(VfsError::IsADirectory),
            NodeContent::BlockDevice { major, minor } => &devfs::get_device(DeviceId {
                dev_type: DeviceType::Block,
                major: *major,
                minor: *minor,
            })
            .expect("Attempted to read from invalid device")
            .read(),
            NodeContent::CharDevice { major, minor } => &devfs::get_device(DeviceId {
                dev_type: DeviceType::Char,
                major: *major,
                minor: *minor,
            })
            .expect("Attempted to read from invalid device")
            .read(),
            _ => return Err(VfsError::InvalidFileType),
        };
        if off > content.len() {
            return Err(VfsError::OffsetTooLarge);
        }
        let len = min(buf.len(), content.len() - off);
        buf[..len].copy_from_slice(&content[off..(off + len)]);
        Ok(len)
    }

    fn write_content(&self, _loc: &FileLocation, off: usize, buf: &[u8]) -> VfsResult<usize> {
        let mut inner = self.0.lock();
        match &mut inner.content {
            NodeContent::Regular(content) => {
                if off > content.len() {
                    return Err(VfsError::OffsetTooLarge);
                }
                let Some(end) = off.checked_add(buf.len()) else {
                    return Err(VfsError::Overflow);
                };
                let new_len = max(content.len(), end);
                content.resize(new_len, 0);
                content[off..end].copy_from_slice(buf);
            }
            NodeContent::Link(content) => {
                content.resize(buf.len(), 0);
                content.copy_from_slice(buf);
            }
            NodeContent::Directory(_) => return Err(VfsError::IsADirectory),
            NodeContent::BlockDevice { major, minor } => {
                devfs::get_device(DeviceId {
                    dev_type: DeviceType::Block,
                    major: *major,
                    minor: *minor,
                })
                .expect("Attempted to write to invalid device")
                .write(off, buf);
            }
            NodeContent::CharDevice { major, minor } => {
                devfs::get_device(DeviceId {
                    dev_type: DeviceType::Char,
                    major: *major,
                    minor: *minor,
                })
                .expect("Attempted to write to invalid device")
                .write(off, buf);
            }
            _ => return Err(VfsError::InvalidFileType),
        }

        Ok(buf.len())
    }

    fn entry_by_name(
        &self,
        loc: &FileLocation,
        name: &str,
    ) -> VfsResult<Option<(DirEntry, Box<dyn VfsNodeOps>)>> {
        let inner = self.0.lock();
        let NodeContent::Directory(entries) = &inner.content else {
            return Err(VfsError::NotADirectory);
        };
        let Some(off) = entries
            .binary_search_by(|ent| ent.name.cmp(&name.to_string()))
            .ok()
        else {
            return Ok(None);
        };
        let ent = entries[off].clone();
        let fs = loc.get_filesystem().unwrap();
        let Some(ops) = fs.node_from_id(ent.id) else {
            return Ok(None);
        };
        Ok(Some((ent, ops)))
    }
}

/// The content of a [Node].
#[derive(Debug)]
enum NodeContent {
    /// A regular file, contains the file's bytes.
    Regular(Vec<u8>),
    /// A directory, contains a list of entries to its children.
    Directory(Vec<DirEntry>),
    /// A link, contains the path to redirect to.
    Link(Vec<u8>),
    /// A pipe, holds nothing
    Fifo,
    /// A socket, holds nothing
    Socket,
    /// A block device, contains the device identifier
    BlockDevice { major: u32, minor: u32 },
    /// A character device, contains the device identifier
    CharDevice { major: u32, minor: u32 },
}

/// The inner content of a tmpfs node
#[derive(Debug)]
struct NodeInner {
    mode: Mode,
    link: u16,
    uid: Uid,
    gid: Gid,
    content: NodeContent,
}

impl NodeInner {
    /// Returns the [Stat] associated with this node
    fn as_stat(&self) -> Stat {
        let (size, dev_major, dev_minor) = match &self.content {
            NodeContent::Regular(content) => (content.len(), 0, 0),
            NodeContent::Directory(_) => (0, 0, 0),
            NodeContent::Link(target) => (target.len(), 0, 0),
            NodeContent::Fifo => (0, 0, 0),
            NodeContent::Socket => (0, 0, 0),
            NodeContent::BlockDevice { major, minor } => (0, *major, *minor),
            NodeContent::CharDevice { major, minor } => (0, *major, *minor),
        };
        Stat::new(
            self.mode, self.link, self.uid, self.gid, size, dev_major, dev_minor,
        )
    }
}
