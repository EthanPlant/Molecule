use alloc::boxed::Box;
use alloc::format;
use alloc::sync::Arc;
use alloc::vec::Vec;

use hashbrown::HashMap;
use spin::{Mutex, Once};

use super::attributes::{FileType, Mode, Stat};
use super::memfs::MemFs;
use super::path::Path;
use super::perm::{AccessProfile, ROOT_GID, ROOT_UID};
use super::vfs::node::VfsNodeOps;
use super::vfs::resolver::{resolve_path, ResolutionSettings, Resolved};
use super::vfs::{self, VfsError, VfsResult};
use super::{FileId, FileSystem};
use crate::drivers::framebuffer::console::ConsoleDev;
use crate::drivers::framebuffer::DevFb;
use crate::fs::mountpoint;

static DEVFS_PATH: &str = "/dev";
static DEVFS: Once<Arc<DevFs>> = Once::new();
static DEVICES: Once<Mutex<HashMap<DeviceId, Arc<dyn Device>>>> = Once::new();

pub struct DevFs(Arc<MemFs>);

impl DevFs {
    pub fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(MemFs::new(false))))
    }
}

impl FileSystem for DevFs {
    fn init(_readonly: bool) -> Arc<dyn FileSystem>
    where
        Self: Sized,
    {
        Self::new()
    }

    fn get_root(&self) -> FileId {
        self.0.get_root()
    }

    fn node_from_id(&self, id: FileId) -> Option<Box<dyn VfsNodeOps>> {
        self.0.node_from_id(id)
    }
}

pub trait Device: Send + Sync {
    /// Get the device id of this device
    fn get_device_id(&self) -> DeviceId;

    /// Get the name of this device
    fn get_name(&self) -> &str;

    fn read(&self) -> Vec<u8>;

    fn write(&self, off: usize, buf: &[u8]) -> usize;
}

pub struct NullDevice;

impl Device for NullDevice {
    fn get_device_id(&self) -> DeviceId {
        DeviceId {
            dev_type: DeviceType::Char,
            major: 1,
            minor: 3,
        }
    }

    fn get_name(&self) -> &str {
        "null"
    }

    fn read(&self) -> Vec<u8> {
        Vec::new()
    }

    fn write(&self, _off: usize, _buf: &[u8]) -> usize {
        0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    Block,
    Char,
}

impl From<DeviceType> for FileType {
    fn from(value: DeviceType) -> Self {
        match value {
            DeviceType::Block => FileType::BlockDevice,
            DeviceType::Char => FileType::CharDevice,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId {
    pub dev_type: DeviceType,
    pub major: u32,
    pub minor: u32,
}

pub fn get_device(id: DeviceId) -> Option<Arc<dyn Device>> {
    let devices = DEVICES.get().expect("Devices not initialized").lock();
    devices.get(&id).cloned()
}

fn register_defaults() -> VfsResult<()> {
    register_device(&(Arc::new(NullDevice {}) as Arc<dyn Device>))?;
    register_device(&(Arc::new(DevFb {}) as Arc<dyn Device>))?;
    register_device(&(Arc::new(ConsoleDev {}) as Arc<dyn Device>))?;
    Ok(())
}

pub fn register_device(device: &Arc<dyn Device>) -> VfsResult<()> {
    let id = device.get_device_id();
    let mut devices = DEVICES.get().expect("Devices not initialized").lock();
    if devices.contains_key(&id) {
        return Err(VfsError::FileAlreadyExists);
    }

    let path = format!("{}/{}", DEVFS_PATH, device.get_name());
    let path = Path::new_unbounded(&path);

    let resolved = resolve_path(
        path,
        &ResolutionSettings {
            create: true,
            ..ResolutionSettings::kernel_no_follow()
        },
    )?;
    match resolved {
        Resolved::Createable { _parent, _name } => {
            vfs::create_file(
                &_parent,
                &_name,
                &AccessProfile::KERNEL,
                Stat::new(
                    Mode::new(FileType::from(id.dev_type), 0o666),
                    0,
                    ROOT_UID,
                    ROOT_GID,
                    0,
                    id.major,
                    id.minor,
                ),
            )?;
            devices.insert(id, device.clone());
            log::debug!("Registered device {} with id {:?}", device.get_name(), id);
        }
        Resolved::Found(_) => return Err(VfsError::FileAlreadyExists),
    };
    Ok(())
}

pub fn init() {
    log::debug!("Initializing devfs");
    let devfs = DevFs::new();

    // Create the devfs mountpoint
    let root = vfs::get_file_from_path(
        Path::new_unbounded("/"),
        &ResolutionSettings::kernel_no_follow(),
    )
    .expect("Cannot find root");
    vfs::create_file(
        &root,
        "dev",
        &AccessProfile::KERNEL,
        Stat::new(
            Mode::new(FileType::Directory, 0o777),
            0,
            ROOT_UID,
            ROOT_GID,
            0,
            0,
            0,
        ),
    )
    .expect("Failed to create devfs directory");
    mountpoint::create(devfs.clone(), DEVFS_PATH).expect("Failed to mount devfs");

    DEVFS.call_once(|| devfs);
    // Initialize the devices map
    let devices = HashMap::new();
    DEVICES.call_once(|| Mutex::new(devices));

    register_defaults().expect("Failed to register default devices");
    log::debug!("devfs initialized");
}
