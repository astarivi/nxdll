use crate::io::storage::path::Path;
use crate::io::storage::vfs::error::VFileSystemError;
use crate::io::storage::vfs::xbox::file::XboxFile;
use crate::io::storage::vfs::xbox::search::{SearchHandle, XboxSearchResult};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use nxdk_rs::sys::winapi::*;
use nxdk_rs::winapi::error::WinError;
use nxdk_rs::winapi::file::{AccessRights, CreationDisposition, FileFlagsAndAttributes, ShareMode, WinFileHandle};

#[derive(Eq, PartialEq, Clone)]
pub struct XboxFileSystem {
    mount_point: char
}

impl XboxFileSystem {
    pub fn new(drive_letter: char) -> Self{
        Self {
            mount_point: drive_letter
        }
    }

    pub fn move_file(&self, from: &Path, to: &Path) -> Result<(), VFileSystemError> {
        let c_from = from.to_xbox(&self.mount_point)?;
        let c_to = to.to_xbox(&self.mount_point)?;

        let success = unsafe {
            MoveFileA(
                c_from.as_ptr() as *const i8,
                c_to.as_ptr() as *const i8
            )
        };

        if success == 0 {
            return Err(WinError::from_last_error().into())
        }

        Ok(())
    }

    pub fn fetch_attributes(&self, path: &Path) -> Result<DWORD, VFileSystemError> {
        let c_from = path.to_xbox(&self.mount_point)?;

        let result = unsafe {
            GetFileAttributesA(
                c_from.as_ptr() as *const i8
            )
        };

        if result == INVALID_FILE_ATTRIBUTES {
            return Err(WinError::from_last_error().into())
        }

        Ok(result)
    }

    pub fn remove_folder(&self, path: &Path) -> Result<(), VFileSystemError>{
        let dir_list = self.list_dir(path)?;

        for search in dir_list{
            if search.metadata().is_dir() {
                self.remove_folder(search.path())?;
            }

            self.delete(&search.path())?;
        }

        let c_path = path.to_xbox(&self.mount_point)?;

        let deleted = unsafe {
            RemoveDirectoryA(
                c_path.as_ptr() as *const i8
            )
        };

        if deleted == 0 {
            return Err(WinError::from_last_error().into())
        }

        Ok(())
    }

    pub fn open(&self, path: &Path, access_rights: AccessRights) -> Result<XboxFile, VFileSystemError> {
        let handle = WinFileHandle::open(
            &path.to_xbox(&self.mount_point)?,
            access_rights.into(),
            ShareMode::Read | ShareMode::Delete,
            CreationDisposition::OpenExisting,
            FileFlagsAndAttributes::AttributeNormal
        )?;

        Ok(XboxFile::new(
            handle,
            path.to_owned()
        ))
    }

    pub fn open_async(&self, path: &Path, access_rights: AccessRights) -> Result<XboxFile, VFileSystemError> {
        let handle = WinFileHandle::open(
            &path.to_xbox(&self.mount_point)?,
            access_rights.into(),
            ShareMode::Read | ShareMode::Delete,
            CreationDisposition::OpenExisting,
            FileFlagsAndAttributes::AttributeNormal | FileFlagsAndAttributes::FlagOverlapped
        )?;

        Ok(XboxFile::new(
            handle,
            path.to_owned()
        ))
    }

    pub fn create(&self, path: &Path, access_rights: AccessRights) -> Result<XboxFile, VFileSystemError> {
        let handle = WinFileHandle::open(
            &path.to_xbox(&self.mount_point)?,
            access_rights.into(),
            ShareMode::Read | ShareMode::Delete,
            CreationDisposition::CreateAlways,
            FileFlagsAndAttributes::AttributeNormal
        )?;

        Ok(XboxFile::new(
            handle,
            path.to_owned()
        ))
    }

    pub fn create_async(&self, path: &Path, access_rights: AccessRights) -> Result<XboxFile, VFileSystemError> {
        let handle = WinFileHandle::open(
            &path.to_xbox(&self.mount_point)?,
            access_rights.into(),
            ShareMode::Read | ShareMode::Delete,
            CreationDisposition::CreateAlways,
            FileFlagsAndAttributes::AttributeNormal | FileFlagsAndAttributes::FlagOverlapped
        )?;

        Ok(XboxFile::new(
            handle,
            path.to_owned()
        ))
    }

    pub fn delete(&self, path: &Path) -> Result<(), VFileSystemError> {
        let c_path = &path.to_xbox(&self.mount_point)?;

        let result = unsafe {
            DeleteFileA(c_path.as_ptr() as *const i8)
        };

        if result == 0 {
            return Err(WinError::from_last_error().into())
        }

        Ok(())
    }

    pub fn create_dir(&self, path: &Path) -> Result<(), VFileSystemError> {
        let c_path = &path.to_xbox(&self.mount_point)?;

        let result = unsafe {
            CreateDirectoryA(
                c_path.as_ptr() as *const i8,
                core::ptr::null_mut()
            )
        };

        if result == 0 {
            return Err(WinError::from_last_error().into())
        }

        Ok(())
    }

    pub fn remove_dir(&self, path: &Path) -> Result<(), VFileSystemError> {
        self.remove_folder(path)
    }

    pub fn list_dir(&self, path: &Path) -> Result<Box<dyn Iterator<Item=XboxSearchResult> + Send>, VFileSystemError> {
        Ok(
            SearchHandle::list_dir(path, &self.mount_point)?
        )
    }

    pub fn exists(&self, path: &Path) -> Result<bool, VFileSystemError> {
        todo!()
    }

    pub fn mount_point(&self) -> &char {
        &self.mount_point
    }
}