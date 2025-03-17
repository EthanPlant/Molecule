//! File system support

use perm::{Gid, Uid};

mod perm;
mod vfs;

/// A filesystem node ID
///
/// An inode is a number representing a node in a filesystem. The kernel doesn't interpret this
/// value in any way, but it must be a unique node in the filesystem.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Inode(u64);

/// A file mode:
/// - Bits 0:11 represent the UNIX file permissions
/// - The remaining bits represent the UNIX file type.
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

/// The different file types supported by the kernel
#[derive(PartialEq, Eq)]
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

/// The location of a file
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileLocation {
    mountpoint_id: u32,
    inode: Inode,
}
/// File status information
struct Stat {
    mode: Mode,

    links: u16,

    uid: Uid,
    gid: Gid,

    size: u64,
    blocks: u64,
}

impl Stat {
    /// Get the file type represented by this status
    pub fn file_type(&self) -> FileType {
        self.mode.file_type()
    }
}
