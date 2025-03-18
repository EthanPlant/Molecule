//! Temporary file system (tmpfs) is a temporary in-memory filesystem.

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use super::perm::{Gid, Uid, ROOT_GID, ROOT_ID};
use super::vfs::node::NodeOps;
use super::vfs::{VfsError, VfsResult};
use super::{
    downcast_fs, DirEntry, FileLocation, FileSystem, FileSystemType, FileType, Inode, Mode, Stat,
    ROOT_INODE,
};
use crate::sync::Mutex;

/// Default maximum amount of memory the tmpfs can use in bytes
const MAX_SIZE: usize = 512 * 1024 * 1024;
/// 512 MiB

/// A temporary filesystem
pub struct TmpFs {
    max_size: usize,
    size: usize,
    readonly: bool,
    nodes: Mutex<NodeStorage>,
}

impl TmpFs {
    pub fn new(max_size: usize, readonly: bool) -> Self {
        let root = Node::new(
            &Stat {
                mode: Mode::from(FileType::Directory) | Mode(0o1777), /* rwx for all groups,
                                                                       * setgid bit set., */
                links: 0,
                uid: ROOT_ID,
                gid: ROOT_GID,
                size: 0,
                blocks: 0,
                dev_major: 0,
                dev_minor: 0,
            },
            Some(ROOT_INODE),
            Some(ROOT_INODE),
        );
        Self {
            max_size,
            size: size_of::<Node>(),
            readonly,
            nodes: Mutex::new(NodeStorage::new(root)),
        }
    }
}

impl FileSystem for TmpFs {
    fn get_root(&self) -> Inode {
        ROOT_INODE
    }

    fn node_from_inode(&self, inode: Inode) -> Option<Box<dyn NodeOps>> {
        let lock = self.nodes.lock();
        let node = lock.get_node(inode)?;
        Some(Box::new(node.clone()))
    }
}

pub struct TmpFsType;

impl FileSystemType for TmpFsType {
    fn load_filesystem(&self, readonly: bool) -> Arc<dyn FileSystem> {
        log::debug!("Initializing tmpfs (readonly={readonly})");
        Arc::new(TmpFs::new(MAX_SIZE, readonly))
    }
}

/// Storage of tmpfs nodes
pub struct NodeStorage(Vec<Option<Node>>);

impl NodeStorage {
    /// Create a new node storeage with the given root node
    pub fn new(root: Node) -> Self {
        Self(vec![Some(root)])
    }

    /// Returns a reference to the node with inode `inode`
    pub fn get_node(&self, inode: Inode) -> Option<&Node> {
        let index = (inode.0 as usize).checked_sub(1)?;
        self.0.get(index).and_then(Option::as_ref)
    }

    /// Returns a free slot for a new node
    pub fn get_free(&mut self) -> (Inode, &mut Option<Node>) {
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
        let inode = Inode(index as u64 + 1);
        let slot = &mut self.0[index];
        (inode, slot)
    }
}

/// A node in the tmpfs
#[derive(Clone, Debug)]
struct Node(Arc<Mutex<NodeInner>>);

impl Node {
    /// Create a new node from the given status.
    pub fn new(stat: &Stat, inode: Option<Inode>, parent: Option<Inode>) -> Self {
        let file_type = stat.file_type();
        let content = match file_type {
            FileType::Regular => NodeContent::Regular(Vec::new()),
            FileType::Directory => {
                let mut entries = Vec::new();
                if let Some(inode) = inode {
                    entries.push(DirEntry {
                        inode,
                        entry_type: FileType::Directory,
                        name: ".".to_string(),
                    });
                };
                if let Some(parent) = parent {
                    entries.push(DirEntry {
                        inode: parent,
                        entry_type: FileType::Directory,
                        name: "..".to_string(),
                    });
                };
                NodeContent::Directory(entries)
            }
            FileType::Link => NodeContent::Link(Vec::new()),
            FileType::Fifo => NodeContent::Fifo,
            FileType::Socket => NodeContent::Socket,
            FileType::BlockDevice => NodeContent::BlockDevice {
                major: stat.dev_major,
                minor: stat.dev_minor,
            },
            FileType::CharDevice => NodeContent::CharDevice {
                major: stat.dev_major,
                minor: stat.dev_minor,
            },
        };
        let mut link = 1;
        if file_type == FileType::Directory {
            link += 1;
        }

        Self(Arc::new(Mutex::new(NodeInner {
            mode: stat.mode,
            link,
            uid: stat.uid,
            gid: stat.gid,
            content,
        })))
    }
}

impl NodeOps for Node {
    fn get_stat(&self, _loc: &FileLocation) -> VfsResult<Stat> {
        let inner = self.0.lock();
        Ok(inner.as_stat())
    }

    fn add_file(
        &self,
        parent: &super::FileLocation,
        name: &str,
        stat: Stat,
    ) -> VfsResult<(Inode, Box<dyn NodeOps>)> {
        log::debug!(
            "tmpfs: Adding file {name} to parent {:?} with status {:?}",
            parent,
            stat
        );
        let fs = parent.get_filesystem().expect("Filesystem is mounted");
        let fs = downcast_fs::<TmpFs>(&*fs);
        if fs.readonly {
            return Err(VfsError::ReadOnly);
        }
        let entry_type = stat.file_type();
        let mut nodes = fs.nodes.lock();
        let (inode, slot) = nodes.get_free();
        log::debug!("tmpfs: Free slot found at inode {:?}", inode);
        let mut parent_inner = self.0.lock();
        let NodeContent::Directory(parent_entries) = &mut parent_inner.content else {
            return Err(VfsError::NotADirectory);
        };
        let node = Node::new(&stat, Some(inode), Some(parent.inode));
        let ent = DirEntry {
            inode,
            entry_type,
            name: name.to_string(),
        };
        let res = parent_entries.binary_search_by(|ent| ent.name.as_str().cmp(name));
        let Err(ent_index) = res else {
            return Err(VfsError::FileAlreadyExists);
        };
        parent_entries.insert(ent_index, ent);
        *slot = Some(node.clone());
        if entry_type == FileType::Directory {
            parent_inner.link += 1;
        }
        Ok((inode, Box::new(node)))
    }
}

/// The content of a [Node]
#[derive(Debug)]
enum NodeContent {
    Regular(Vec<u8>),
    Directory(Vec<DirEntry>),
    Link(Vec<u8>),
    Fifo,
    Socket,
    BlockDevice { major: u32, minor: u32 },
    CharDevice { major: u32, minor: u32 },
}

#[derive(Debug)]
struct NodeInner {
    mode: Mode,
    link: u16,
    uid: Uid,
    gid: Gid,
    content: NodeContent,
}

impl NodeInner {
    /// Returns the [Stat] associated with the content
    fn as_stat(&self) -> Stat {
        let (file_type, size, dev_major, dev_minor) = match &self.content {
            NodeContent::Regular(content) => (FileType::Regular, content.len() as _, 0, 0),
            NodeContent::Directory(_) => (FileType::Directory, 0, 0, 0),
            NodeContent::Link(target) => (FileType::Link, target.len() as _, 0, 0),
            NodeContent::Fifo => (FileType::Fifo, 0, 0, 0),
            NodeContent::Socket => (FileType::Socket, 0, 0, 0),
            NodeContent::BlockDevice { major, minor } => (FileType::BlockDevice, 0, *major, *minor),
            NodeContent::CharDevice { major, minor } => (FileType::CharDevice, 0, *major, *minor),
        };
        Stat {
            mode: Mode::from(file_type) | self.mode,
            links: self.link,
            uid: self.uid,
            gid: self.gid,
            size,
            blocks: size / 0x1000,
            dev_major,
            dev_minor,
        }
    }
}
