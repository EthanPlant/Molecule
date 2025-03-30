use alloc::sync::Arc;

use hashbrown::HashMap;
use spin::Once;

use super::entry::VfsEntry;
use super::node::{self, VfsNode};
use crate::fs::memfs::MemFs;
use crate::fs::{FileLocation, FileSystem};
use crate::sync::Mutex;

static MOUNT_POINTS: Once<Mutex<HashMap<u32, Arc<MountPoint>>>> = Once::new();

/// A mountpoint, enabling attaching a filesystem to a directory in the VFS
pub struct MountPoint {
    _id: u32,
    _flags: u32,
    fs: Arc<dyn FileSystem>,
    _root: Arc<VfsEntry>,
}

impl MountPoint {
    /// Get the filesystem for this mountpoint
    pub fn filesystem(&self) -> Arc<dyn FileSystem> {
        self.fs.clone()
    }
}

/// Get a mountpoint from an id
pub fn from_id(id: u32) -> Option<Arc<MountPoint>> {
    MOUNT_POINTS
        .get()
        .expect("Filesystem is initialized")
        .lock()
        .get(&id)
        .cloned()
}

/// Create the root mountpoint
pub fn create_root() -> Arc<VfsEntry> {
    let tmp = MemFs::init(false);
    let root = tmp.get_root();
    let node = node::insert(VfsNode::new(
        FileLocation {
            mountpoint_id: 0,
            file_id: root,
        },
        tmp.node_from_id(root).unwrap(),
    ));
    let root_entry = Arc::new(VfsEntry::new(node, "/", None));
    let mount_point = Arc::new(MountPoint {
        _id: 0,
        _flags: 0,
        fs: tmp,
        _root: root_entry.clone(),
    });
    let mut mountpoints = HashMap::new();
    mountpoints.insert(0, mount_point.clone());
    MOUNT_POINTS.call_once(|| Mutex::new(mountpoints));
    root_entry
}
