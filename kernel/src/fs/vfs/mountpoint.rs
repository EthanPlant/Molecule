use alloc::sync::Arc;
use core::sync::atomic::AtomicU32;

use hashbrown::HashMap;
use spin::Once;

use super::entry::{EntryChild, VfsEntry};
use super::node::{self, VfsNode};
use super::resolver::ResolutionSettings;
use super::{get_file_from_path, VfsResult};
use crate::fs::memfs::MemFs;
use crate::fs::path::Path;
use crate::fs::{FileLocation, FileSystem};
use crate::sync::Mutex;

static MOUNT_POINT_ID: AtomicU32 = AtomicU32::new(1);

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

/// Create a new mountpoint at the given path.
pub fn create(fs: Arc<dyn FileSystem>, path: &str) -> VfsResult<()> {
    let parent_path = Path::new(path)?
        .parent()
        .expect("Mountpoint must have a parent");
    let parent = get_file_from_path(parent_path, &ResolutionSettings::kernel_no_follow())?;
    let id = MOUNT_POINT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let root_id = fs.get_root();
    let node = node::insert(VfsNode::new(
        FileLocation {
            mountpoint_id: id,
            file_id: root_id,
        },
        fs.node_from_id(root_id).unwrap(),
    ));
    let root_entry = Arc::new(VfsEntry::new(node, path, Some(parent.clone())));
    let mount_point = Arc::new(MountPoint {
        _id: id,
        _flags: 0,
        fs,
        _root: root_entry.clone(),
    });
    let mut mountpoints = MOUNT_POINTS
        .get()
        .expect("Filesystem is initialized")
        .lock();
    mountpoints.insert(id, mount_point.clone());
    parent.children().insert(EntryChild::new(root_entry));
    Ok(())
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
