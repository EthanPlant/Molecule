//! File system support

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use core::any::Any;
use core::ops::BitOr;

use perm::{Gid, Uid, ROOT_GID, ROOT_ID};
use vfs::mountpoint::{self, MountPoint};
use vfs::node::NodeOps;

pub mod perm;
mod tmpfs;
pub mod vfs;

const ROOT_INODE: Inode = Inode(1);

/// A filesystem type.
pub trait FileSystemType {
    /// Load an initialize a filesystem.
    fn load_filesystem(&self, readonly: bool) -> Arc<dyn FileSystem>;
}

pub trait FileSystem: Any + Send + Sync {
    fn get_root(&self) -> Inode;

    fn node_from_inode(&self, inode: Inode) -> Option<Box<dyn NodeOps>>;
}

/// Downcasts the given `fs` into `F`.
///
/// # Panics
///
/// This function panics if `fs` and `F` do not match
pub fn downcast_fs<F: FileSystem>(fs: &dyn FileSystem) -> &F {
    (fs as &dyn Any).downcast_ref().unwrap()
}

/// A filesystem node ID
///
/// An inode is a number representing a node in a filesystem. The kernel doesn't interpret this
/// value in any way, but it must be a unique node in the filesystem.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
struct Inode(u64);

/// A file mode:
/// - Bits 0:11 represent the UNIX file permissions
/// - The remaining bits represent the UNIX file type.
#[derive(Clone, Copy, Debug)]
struct Mode(u32);

impl Mode {
    /// File type: Block device
    const S_IFBLK: u32 = 0o060000;
    /// File type: Character device
    const S_IFCHR: u32 = 0o020000;
    /// File type: Directory
    const S_IFDIR: u32 = 0o040000;
    /// File type: Fifo
    const S_IFIFO: u32 = 0o010000;
    /// File type: Symbolic link
    const S_IFLINK: u32 = 0o120000;
    /// File type: Regular file
    const S_IFREG: u32 = 0o100000;
    /// File type: Socket
    const S_IFSOCK: u32 = 0o140000;

    /// Get the filetype represented by this mode
    pub fn file_type(&self) -> FileType {
        match self.0 & 0o770000 {
            Self::S_IFREG => FileType::Regular,
            Self::S_IFDIR => FileType::Directory,
            Self::S_IFLINK => FileType::Link,
            Self::S_IFIFO => FileType::Fifo,
            Self::S_IFSOCK => FileType::Socket,
            Self::S_IFBLK => FileType::BlockDevice,
            Self::S_IFCHR => FileType::CharDevice,
            _ => unreachable!("Invalid file type"),
        }
    }

    /// Check if the mode has a given permission
    pub fn has_permission(&self, permission: u32) -> bool {
        self.0 & permission != 0
    }
}

impl BitOr for Mode {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// The different file types supported by the kernel
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FileType {
    /// A regular file for storing data.
    Regular,
    /// A directory, containing directory entries to other files.
    Directory,
    /// A symbolic link, pointing to another file.
    Link,
    /// A nameed pipe.
    Fifo,
    /// A Unix socket.
    Socket,
    /// A block device file.
    BlockDevice,
    /// A character device file.
    CharDevice,
}

impl From<FileType> for Mode {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Regular => Self(Self::S_IFREG),
            FileType::Directory => Self(Self::S_IFDIR),
            FileType::Link => Self(Self::S_IFLINK),
            FileType::Fifo => Self(Self::S_IFIFO),
            FileType::Socket => Self(Self::S_IFSOCK),
            FileType::BlockDevice => Self(Self::S_IFBLK),
            FileType::CharDevice => Self(Self::S_IFCHR),
        }
    }
}

/// The location of a file
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FileLocation {
    mountpoint_id: u32,
    inode: Inode,
}

impl FileLocation {
    pub fn get_mountpoint(&self) -> Option<Arc<MountPoint>> {
        mountpoint::from_id(self.mountpoint_id)
    }

    pub fn get_filesystem(&self) -> Option<Arc<dyn FileSystem>> {
        self.get_mountpoint().map(|mp| mp.filesystem())
    }
}

/// File status information
#[derive(Debug)]
pub struct Stat {
    mode: Mode,

    links: u16,

    uid: Uid,
    gid: Gid,

    size: u64,
    blocks: u64,

    dev_major: u32,
    dev_minor: u32,
}

impl Stat {
    pub fn new() -> Self {
        Self {
            mode: Mode::from(FileType::Regular),
            links: 0,
            uid: ROOT_ID,
            gid: ROOT_GID,
            size: 0,
            blocks: 0,
            dev_major: 0,
            dev_minor: 0,
        }
    }

    /// Get the file type represented by this status
    pub fn file_type(&self) -> FileType {
        self.mode.file_type()
    }
}

/// An entry in a directory
#[derive(Debug)]
struct DirEntry {
    inode: Inode,
    entry_type: FileType,
    name: String,
}

/// Initialize file management
pub fn init() {
    log::debug!("fs: Initializing filesystem");
    let root = mountpoint::create_root();
    vfs::ROOT.call_once(|| root);
    log::debug!("fs: Filesystem initialized");
}
