//! Filesystem file attributes

use super::perm::{Gid, Uid};
use super::vfs::{VfsError, VfsResult};

/// File types.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FileType {
    /// A regular file for storing data.
    Regular,
    /// A directory, containing [directory entries](super::DirEntry) to other files.
    Directory,
    /// A symbolic link, which points to another file.
    Link,
    /// A named pipe used for interprocess communication.
    Fifo,
    /// A socket, used for IPC or network communication.
    Socket,
    /// A block device file, such as a disk.
    BlockDevice,
    /// A character device file, such as a terminal.
    CharDevice,
}

impl From<FileType> for u32 {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Regular => Mode::S_ISREG,
            FileType::Directory => Mode::S_ISDIR,
            FileType::Link => Mode::S_ISLINK,
            FileType::Fifo => Mode::S_ISFIFO,
            FileType::Socket => Mode::S_ISSOCK,
            FileType::BlockDevice => Mode::S_ISBLK,
            FileType::CharDevice => Mode::S_ISCHR,
        }
    }
}

/// A file's mode information contains two types of data, the file's file type and the file's access
/// permission.
#[derive(Clone, Copy, Debug)]
pub struct Mode(u32);

impl Mode {
    const S_ISBLK: u32 = 0o060000;
    const S_ISCHR: u32 = 0o020000;
    const S_ISDIR: u32 = 0o040000;
    const S_ISFIFO: u32 = 0o010000;
    const S_ISLINK: u32 = 0o120000;
    const S_ISREG: u32 = 0o100000;
    const S_ISSOCK: u32 = 0o140000;

    /// Create a new mode from a file type and permissions
    pub fn new(file_type: FileType, permissions: u32) -> Self {
        Self(u32::from(file_type) | permissions)
    }

    /// Create a new mode from a u32
    pub fn from_u32(mode: u32) -> Self {
        Self(mode)
    }

    /// Get the file type represented by this mode.
    ///
    /// # Errors
    ///
    /// This function returns [VfsError::InvalidFileType] if the mode represents a non-existent file
    /// type.
    fn file_type(&self) -> VfsResult<FileType> {
        match self.0 & 0o770000 {
            Self::S_ISBLK => Ok(FileType::BlockDevice),
            Self::S_ISCHR => Ok(FileType::CharDevice),
            Self::S_ISDIR => Ok(FileType::Directory),
            Self::S_ISFIFO => Ok(FileType::Fifo),
            Self::S_ISLINK => Ok(FileType::Link),
            Self::S_ISREG => Ok(FileType::Regular),
            Self::S_ISSOCK => Ok(FileType::Socket),
            _ => Err(VfsError::InvalidFileType),
        }
    }

    /// Check if the mode has given permissions
    pub fn has_permission(&self, permission: u32) -> bool {
        self.0 & permission != 0
    }
}

/// File status information, containing all of the metadata about the file.
pub struct Stat {
    mode: Mode,

    _link: u16,

    user: Uid,
    group: Gid,

    size: usize,
    _blocks: usize,

    dev_major: u32,
    dev_minor: u32,
}

impl Stat {
    /// Create a new stat
    pub fn new(
        mode: Mode,
        link: u16,
        uid: Uid,
        gid: Gid,
        size: usize,
        dev_major: u32,
        dev_minor: u32,
    ) -> Self {
        Self {
            mode,
            _link: link,
            user: uid,
            group: gid,
            size,
            _blocks: size / 4096,
            dev_major,
            dev_minor,
        }
    }

    /// Get the size of the file in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the file type of the file
    pub fn file_type(&self) -> VfsResult<FileType> {
        self.mode.file_type()
    }

    /// Get the mode for the file
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Get the user id for this file
    pub fn uid(&self) -> Uid {
        self.user
    }

    /// Set the user id for this file
    pub fn set_user(&mut self, uid: Uid) {
        self.user = uid;
    }

    /// Get the group id for this file
    pub fn gid(&self) -> Gid {
        self.group
    }

    /// Set the group id for this file
    pub fn set_group(&mut self, gid: Gid) {
        self.group = gid
    }

    /// Get the device major identifier for this file
    pub fn dev_major(&self) -> u32 {
        self.dev_major
    }

    /// Get the device minor for this file
    pub fn dev_minor(&self) -> u32 {
        self.dev_minor
    }
}
