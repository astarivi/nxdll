use crate::io::storage::vfs::xbox::fs::XboxFileSystem;
use alloc::boxed::Box;
use alloc::vec::Vec;
use nxdk_rs::nxdk::mount::{nx_is_drive_mounted, nx_mount_drive, nx_mount_execution_to, nx_unmount_drive};
use nxdk_rs::sys::winapi::GetLogicalDrives;

pub struct XboxStorage {
    hdd0: &'static XboxHardDisk,
    hdd1: Option<&'static XboxHardDisk>,
}

impl XboxStorage {
    pub fn hdd0(&self) -> &'static XboxHardDisk {
        self.hdd0
    }

    pub fn hdd1(&self) -> Option<&'static XboxHardDisk> {
        self.hdd1
    }

    pub fn internal_from_name(&self, name: &str) -> Option<&'static XboxHardDisk> {
        if name == self.hdd0.device_name {
            return Some(self.hdd0);
        }

        if let Some(hdd1) = self.hdd1 {
            if name == hdd1.device_name {
                return Some(hdd1);
            }
        }

        None
    }

    pub fn internal_devices(&self) -> Vec<&'static XboxHardDisk> {
        let mut devices: Vec<&XboxHardDisk> = Vec::new();
        devices.push(&self.hdd0);

        if let Some(hdd1) = self.hdd1 {
            devices.push(hdd1);
        }

        devices
    }
}

#[derive(Eq, PartialEq, Clone)]
pub struct XboxHardDisk {
    device_name: &'static str,
    logical_mounts: Box<[&'static XboxFileSystem]>
}

impl XboxHardDisk {
    pub fn device_name(&self) -> &str {
        self.device_name
    }

    pub fn logical_mounts(&self) -> &Box<[&'static XboxFileSystem]> {
        &self.logical_mounts
    }

    pub fn fs_from_mount(&self, required_mount: char) -> Option<&'static XboxFileSystem> {
        for mount in self.logical_mounts.iter() {
            if required_mount == *mount.mount_point() {
                return Some(mount);
            }
        }

        None
    }
}

/// Mount all available partitions, and mount execution directory to Q partition.
/// In most cases, mount_platform_storage() shouldn't be used, as get_storage()
/// calls this. And get_storage() is static lazy init as INTERNAL_STORAGE pub
/// static.
///
/// # Example:
///
/// ```
/// use nxdk_rs::hal::debug::debug_print_str_ln;
/// use crate::io::storage::mount::INTERNAL_STORAGE;
///
/// debug_print_str_ln(&INTERNAL_STORAGE.hdd0.device_name);
/// ```
pub fn mount_platform_storage() {
    if nx_is_drive_mounted('D') {
        nx_unmount_drive('D');
    }

    nx_mount_execution_to('Q');

    // Basic partitions
    nx_mount_drive('C', "\\Device\\Harddisk0\\Partition2\\");
    nx_mount_drive('E', "\\Device\\Harddisk0\\Partition1\\");
    nx_mount_drive('X', "\\Device\\Harddisk0\\Partition3\\");
    nx_mount_drive('Y', "\\Device\\Harddisk0\\Partition4\\");
    nx_mount_drive('Z', "\\Device\\Harddisk0\\Partition5\\");

    // Mount extended partitions
    nx_mount_drive('F', "\\Device\\Harddisk0\\Partition6\\");
    nx_mount_drive('G', "\\Device\\Harddisk0\\Partition7\\");
    nx_mount_drive('R', "\\Device\\Harddisk0\\Partition8\\");
    nx_mount_drive('S', "\\Device\\Harddisk0\\Partition9\\");
    nx_mount_drive('V', "\\Device\\Harddisk0\\Partition10\\");
    nx_mount_drive('W', "\\Device\\Harddisk0\\Partition11\\");
    nx_mount_drive('A', "\\Device\\Harddisk0\\Partition12\\");
    nx_mount_drive('B', "\\Device\\Harddisk0\\Partition13\\");
    nx_mount_drive('P', "\\Device\\Harddisk0\\Partition14\\");

    // Mount HDD2 partitions
    nx_mount_drive('H', "\\Device\\Harddisk1\\Partition1\\");
    nx_mount_drive('I', "\\Device\\Harddisk1\\Partition2\\");
    nx_mount_drive('J', "\\Device\\Harddisk1\\Partition3\\");
    nx_mount_drive('K', "\\Device\\Harddisk1\\Partition4\\");
    nx_mount_drive('L', "\\Device\\Harddisk1\\Partition5\\");
    
}

/// Initializes all console storage, and returns a structure to interact it
///
/// Actual memory leak extravaganza 草
pub fn get_storage() -> XboxStorage {
    mount_platform_storage();

    let mut hdd0 = vec!();
    let mut hdd1 = vec!();

    let drive_mask = unsafe { GetLogicalDrives() };
    for i in 0..26 {
        if (drive_mask & (1 << i)) == 0 {
            continue;
        }

        let drive_letter: char = ('A' as u8 + i as u8) as char;

        let fs = Box::new(XboxFileSystem::new(drive_letter));

        match drive_letter {
            'H' | 'I' | 'J' | 'K' | 'L' => {
                hdd1.push(Box::leak(fs) as &'static XboxFileSystem)
            }
            _ => {
                hdd0.push(Box::leak(fs) as &'static XboxFileSystem)
            }
        }
    }

    let device0 = Box::new(XboxHardDisk {
        device_name: "Harddisk0",
        logical_mounts: hdd0.into_boxed_slice()
    });

    let device1 = if hdd1.is_empty() {
        None
    } else {
        let hdd1 = Box::new(XboxHardDisk {
            device_name: "Harddisk1",
            logical_mounts: hdd1.into_boxed_slice()
        });

        Some(Box::leak(hdd1) as &'static XboxHardDisk)
    };

    XboxStorage {
        hdd0: Box::leak(device0),
        hdd1: device1,
    }
}
