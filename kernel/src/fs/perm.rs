//! File access permission handling.

use super::attributes::Stat;

/// Read permission for file owner
pub const S_IRUSR: u32 = 0o0400;
/// Execute permission for file owner
pub const S_IXUSR: u32 = 0o0100;
/// Write permission for file owner
pub const S_IWUSR: u32 = 0o0200;

/// Read permission for file group owner
pub const S_IRGRP: u32 = 0o0040;
/// Execute permission for file group owner
pub const S_IWGRP: u32 = 0o0020;
/// Write permission for file group owner
pub const S_IXGRP: u32 = 0o0010;

/// Read permission for all other users
pub const S_IROTH: u32 = 0o0004;
/// Write permission for all other users
pub const S_IWOTH: u32 = 0o0002;
/// Execute permission for all other users
pub const S_IXOTH: u32 = 0o0001;

/// Set GID. If set, children inherit the group ID of their parent directory. Otherwise the group id
/// of the access profile is used.
pub const S_ISGID: u32 = 0o2000;

/// User id used by the root user or kernel, granting access to all files regardless of permission.
pub const ROOT_UID: Uid = Uid(0);

/// Group id used by the root user or kernel, granting access to all files regardless of permission
pub const ROOT_GID: Gid = Gid(0);

/// A user id, representing a user that owns or is accessing a file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Uid(u16);

impl Uid {
    /// Create a new user id
    pub const fn new(id: u16) -> Self {
        Self(id)
    }
}

/// A group id, representing a group that owns or is accessing a file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Gid(u16);

impl Gid {
    /// Create a new group id
    pub const fn new(id: u16) -> Self {
        Self(id)
    }
}

/// A set of information determining whether an agent can access a resource
#[derive(Clone, Copy)]
pub struct AccessProfile {
    uid: Uid,
    gid: Gid,

    effective_uid: Uid,
    effective_gid: Gid,

    _saved_uid: Uid,
    _saved_gid: Gid,
}

impl AccessProfile {
    /// Access profile used by the kernel, granting access to all files regardless of permission.
    pub const KERNEL: Self = Self {
        uid: ROOT_UID,
        gid: ROOT_GID,
        effective_uid: ROOT_UID,
        effective_gid: ROOT_GID,
        _saved_uid: ROOT_UID,
        _saved_gid: ROOT_GID,
    };

    /// Get the effective user id for this access profile
    pub fn effective_uid(&self) -> Uid {
        self.effective_uid
    }

    /// Get the effective group id for this access profile
    pub fn effective_gid(&self) -> Gid {
        self.effective_gid
    }

    /// Checks whether the access profile can execute a file with a given status.
    pub fn can_execute_file(&self, stat: &Stat) -> bool {
        self.check_execute(stat, true)
    }

    /// Checks whether the access profile can modify a file with a given status.
    pub fn can_write_file(&self, stat: &Stat) -> bool {
        self.check_write(stat, true)
    }

    /// Checks whether the access profile can modify entries in a directory with a given status. In
    /// order to modify a directory the access profile must have write and execute permissions.
    pub fn can_write_dir(&self, stat: &Stat) -> bool {
        self.can_write_file(stat) && self.can_execute_file(stat)
    }

    /// Checks whether the access profile can search a directory with a given status. In order to
    /// search a directory the access profile must have execute permissions.
    pub fn can_search_directory(&self, stat: &Stat) -> bool {
        self.can_execute_file(stat)
    }

    /// Checks if the access profile can execute a file with the given status.
    fn check_execute(&self, stat: &Stat, effective: bool) -> bool {
        let (uid, gid) = self.get_id(effective);

        if uid == ROOT_UID || gid == ROOT_GID {
            return true;
        }

        if stat.mode().has_permission(S_IXUSR) && stat.uid() == uid {
            return true;
        }

        if stat.mode().has_permission(S_IXGRP) && stat.gid() == gid {
            return true;
        }

        stat.mode().has_permission(S_IXOTH)
    }

    /// Checks if the access profile can write to a file with the given status.
    fn check_write(&self, stat: &Stat, effective: bool) -> bool {
        let (uid, gid) = self.get_id(effective);

        if uid == ROOT_UID || gid == ROOT_GID {
            return true;
        }

        if stat.mode().has_permission(S_IWUSR) && stat.uid() == uid {
            return true;
        }

        if stat.mode().has_permission(S_IWGRP) && stat.gid() == gid {
            return true;
        }

        stat.mode().has_permission(S_IWOTH)
    }

    /// Get the user and group ids to use for permission checking.
    fn get_id(&self, effective: bool) -> (Uid, Gid) {
        if effective {
            (self.effective_uid, self.effective_gid)
        } else {
            (self.uid, self.gid)
        }
    }
}
