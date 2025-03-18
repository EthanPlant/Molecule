use alloc::string::String;
use alloc::sync::Arc;

use hashbrown::HashMap;
use spin::Lazy;

use super::node::{self, Node};
use super::Entry;
use crate::fs::tmpfs::{TmpFs, TmpFsType};
use crate::fs::{FileLocation, FileSystem, FileSystemType};
use crate::sync::Mutex;

static MOUNT_POINTS: Lazy<Mutex<HashMap<u32, Arc<MountPoint>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// A mount point, enabling attaching a filesystem to a directory in the VFS
pub struct MountPoint {
    id: u32,
    flags: u32,
    fs: Arc<dyn FileSystem>,
    root: Arc<super::Entry>,
}

impl MountPoint {
    pub fn filesystem(&self) -> Arc<dyn FileSystem> {
        self.fs.clone()
    }
}

/// Creates the root mountpoint, and returns the newly created root entry of the VFS
pub fn create_root() -> Arc<super::Entry> {
    let tmp = TmpFsType;
    let tmp = tmp.load_filesystem(false);
    let root = tmp.get_root();
    let node = node::insert(Node::new(
        FileLocation {
            mountpoint_id: 0,
            inode: root,
        },
        tmp.node_from_inode(root).unwrap(),
    ));
    let root_entry = Arc::new(Entry::new(node));
    let mountpoint = Arc::new(MountPoint {
        id: 0,
        flags: 0,
        fs: tmp,
        root: root_entry.clone(),
    });
    MOUNT_POINTS.lock().insert(0, mountpoint);
    root_entry
}

pub fn from_id(id: u32) -> Option<Arc<MountPoint>> {
    MOUNT_POINTS.lock().get(&id).cloned()
}
