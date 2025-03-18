//! Implementation of UNIX file permissions

use super::{Mode, Stat};

/// The root user ID
pub const ROOT_ID: Uid = Uid(0);
/// The root group ID
pub const ROOT_GID: Gid = Gid(0);

/// Type representing a user ID
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Uid(u16);
/// Type representing a group ID
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gid(u16);

/// User: Write
const S_IWUSR: u32 = 0o0200;
/// User: Execute
const S_IXUSR: u32 = 0o0100;

/// Group: Write
const S_IWGRP: u32 = 0o0020;
/// Group: Execute
const S_IXGRP: u32 = 0o0010;

/// Other: Write
const S_IWOTH: u32 = 0o0002;
/// Other: Execute
const S_IXOTH: u32 = 0o0001;

/// Set GID
pub const S_ISGID: u32 = 0o2000;

/// A set of information to determine if an agent can access a resource.
pub struct AccessProfile {
    uid: Uid,
    gid: Gid,
    euid: Uid,
    egid: Gid,
    suid: Uid,
    sgid: Gid,
}

impl AccessProfile {
    pub const KERN_PROFILE: Self = Self {
        uid: ROOT_ID,
        gid: ROOT_GID,
        euid: ROOT_ID,
        egid: ROOT_GID,
        suid: ROOT_ID,
        sgid: ROOT_GID,
    };

    /// Get the effective UID of the agent
    pub fn effective_uid(&self) -> Uid {
        self.euid
    }

    /// Get the effective GID of the agent
    pub fn effective_gid(&self) -> Gid {
        self.egid
    }

    /// Checks whether the agent can write to a file with the given status
    pub fn can_write_file(&self, stat: &Stat) -> bool {
        self.check_write_access(stat, true)
    }

    /// Checks whether the agent can modify entries in a directory with the given status
    pub fn can_write_dir(&self, stat: &Stat) -> bool {
        self.can_write_file(stat) && self.can_execute_file(stat)
    }

    /// Checks whether the agent can execute a file with given status.
    pub fn can_execute_file(&self, stat: &Stat) -> bool {
        self.check_execute_access(stat, true)
    }

    /// Tells whether the agent can write to a file with the given status. `effective` tells whether
    /// to use effective IDs, otherwise real IDs are used
    fn check_write_access(&self, stat: &Stat, effective: bool) -> bool {
        let (uid, gid) = self.get_id(effective);

        if uid == ROOT_ID || gid == ROOT_GID {
            return true;
        }

        if stat.mode.has_permission(S_IWUSR) && stat.uid == uid {
            return true;
        }

        if stat.mode.has_permission(S_IWGRP) && stat.gid == gid {
            return true;
        }

        stat.mode.has_permission(S_IWOTH)
    }

    /// Tells whether the agent can exevute a file with the given stats. `effective` tells whether
    /// to use effective IDs, otherwise real IDs are used
    fn check_execute_access(&self, stat: &Stat, effective: bool) -> bool {
        let (uid, gid) = self.get_id(effective);

        if uid == ROOT_ID || gid == ROOT_GID {
            return true;
        }

        if stat.mode.has_permission(S_IXUSR) && stat.uid == uid {
            return true;
        }

        if stat.mode.has_permission(S_IXGRP) && stat.gid == gid {
            return true;
        }

        stat.mode.has_permission(S_IXOTH)
    }

    fn get_id(&self, effective: bool) -> (Uid, Gid) {
        if effective {
            (self.euid, self.egid)
        } else {
            (self.uid, self.gid)
        }
    }
}
