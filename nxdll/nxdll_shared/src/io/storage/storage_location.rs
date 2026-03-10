use crate::io::storage::mount::XboxHardDisk;
use crate::io::storage::path::{Path, PathError};
use crate::io::storage::vfs::error::VFileSystemError;
use crate::io::storage::vfs::xbox::file::{XboxFile, XboxFileMetadata};
use crate::io::storage::vfs::xbox::fs::XboxFileSystem;
use crate::io::storage::vfs::xbox::search::XboxSearchResult;
use crate::io::INTERNAL_STORAGE;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use core::error::Error;
use core::fmt;
use nxdk_rs::sys::pbkit::FILE_ATTRIBUTE_NORMAL;
use nxdk_rs::sys::winapi::FILE_ATTRIBUTE_DIRECTORY;
use nxdk_rs::winapi::file::{AccessRights, FileStandardInformation};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LocalLocationError {
    PathError(PathError),
    FsError(VFileSystemError),
    NotFound,
    ReadOnly,
    MalformedPath
}

impl From<PathError> for LocalLocationError {
    fn from(e: PathError) -> Self {
        LocalLocationError::PathError(e)
    }
}

impl From<VFileSystemError> for LocalLocationError {
    fn from(e: VFileSystemError) -> Self {
        LocalLocationError::FsError(e)
    }
}

impl fmt::Display for LocalLocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for LocalLocationError {}

// TODO: Perhaps it would be better to have a general usage class doing this?
// FIXME: After TODO, this whole class should wrap around generic traits and dynamic dispatch
/// FtpPath wraps around different storage devices and storage systems, allowing a single object
/// to represent a path in a given device and file system.
///
/// The rationale behind this kind of path is to support:
/// 1. Multiple filesystems (or partitions, as they are often called)
/// 2. Multiple devices (hard drives, USB MS, etc...)
/// 3. All this while keeping a single, unified object for access.
pub struct StorageLocation {
    cwd: Option<Path>,
    device: Option<&'static XboxHardDisk>,
    fs: Option<&'static XboxFileSystem>
}

impl StorageLocation {
    pub fn new() -> Self {
        Self {
            cwd: None,
            device: None,
            fs: None
        }
    }

    pub fn resolve(&self, path: &str) -> Result<Self, LocalLocationError> {
        if path.starts_with("/") {
            Self::resolve_generic(
                None,
                None,
                None,
                path
            )
        } else {
            Self::resolve_generic(
                self.device,
                self.fs,
                self.cwd.clone(),
                path
            )
        }
    }

    pub fn resolve_absolute(path: &str) -> Result<Self, LocalLocationError> {
        Self::resolve_generic(
            None,
            None,
            None,
            path
        )
    }

    fn resolve_generic(
        d: Option<&'static XboxHardDisk>,
        f: Option<&'static XboxFileSystem>,
        c: Option<Path>,
        path: &str
    ) -> Result<Self, LocalLocationError> {
        let mut device: Option<&'static XboxHardDisk> = d;
        let mut fs: Option<&'static XboxFileSystem> = f;
        let mut cwd: Option<Path> =c;

        let path_components = path.split('/')
            .filter(|component| !component.is_empty());

        for component in path_components {
            let recurse_up = component == "..";

            if device.is_none() {
                if recurse_up {
                    return Err(PathError::RootPop.into())
                }

                device = Some(
                    INTERNAL_STORAGE.internal_from_name(component).ok_or(LocalLocationError::NotFound)?
                );
                continue;
            }

            if fs.is_none() {
                if recurse_up {
                    device = None;
                    cwd = None;
                    continue;
                }

                fs = Some(
                    device.as_ref().unwrap().fs_from_mount(
                        component.chars().next().ok_or(LocalLocationError::MalformedPath)?
                    ).ok_or(LocalLocationError::NotFound)?
                );

                cwd = Some(Path::new("/")?);
                continue;
            }

            if cwd.is_none() {
                cwd = Some(Path::new("/")?);
            }

            let current_cwd = cwd.as_ref().unwrap();

            if current_cwd.as_str()? == "/" && recurse_up{
                fs = None;
                cwd = None;
                continue;
            }

            if recurse_up {
                cwd = Some(current_cwd.pop()?);
            } else {
                cwd = Some(current_cwd.push(component)?);
            }
        }

        Ok(Self {
            device,
            fs,
            cwd
        })
    }

    pub fn get_cwd(&self) -> Option<&Path> {
        self.cwd.as_ref()
    }

    pub fn get_device(&self) -> Option<&'static XboxHardDisk> {
        self.device.map(|v| v)
    }

    pub fn get_fs(&self) -> Option<&'static XboxFileSystem> {
        self.fs.map(|v| v)
    }

    fn check_fs_cwd(&self) -> Result<(&XboxFileSystem, &Path), LocalLocationError> {
        if self.is_read_only()? {
            return Err(LocalLocationError::ReadOnly)
        }

        match self.fs.as_ref().zip(self.cwd.as_ref()) {
            None => Err(LocalLocationError::MalformedPath),
            Some((fs, path)) => {
                Ok((fs, path))
            }
        }
    }

    pub fn is_root(&self) -> bool {
        self.device.is_none()
    }

    pub fn is_read_only(&self) -> Result<bool, LocalLocationError> {
        Ok(
            match self.cwd.as_ref() {
                None => true,
                Some(cwd) => {
                    cwd.as_str()? == "/"
                }
            }
        )
    }

    pub fn get_pwd(&self) -> Result<String, LocalLocationError> {
        if self.device.is_none() {
            return Ok("/".to_owned());
        }

        if self.fs.is_none() {
            return Ok(format!("/{}", self.device.as_ref().unwrap().device_name()));
        }

        let pwd = match &self.cwd {
            None => {""}
            Some(awd) => awd.as_str()?
        };

        Ok(format!(
            "/{}/{}{}",
            self.device.as_ref().unwrap().device_name(),
            self.fs.as_ref().unwrap().mount_point(),
            pwd
        ))
    }

    pub fn create_dir(&self) -> Result<(), LocalLocationError> {
        let (fs, path) = self.check_fs_cwd()?;

        fs.create_dir(path)?;
        Ok(())
    }

    pub fn remove_folder(&self) -> Result<(), LocalLocationError> {
        let (fs, path) = self.check_fs_cwd()?;

        fs.remove_folder(path)?;
        Ok(())
    }

    pub fn delete_file(&self) -> Result<(), LocalLocationError> {
        let (fs, path) = self.check_fs_cwd()?;

        fs.delete(path)?;
        Ok(())
    }

    pub fn open(&self, flags: AccessRights) -> Result<XboxFile, LocalLocationError> {
        let (fs, path) = self.check_fs_cwd()?;

        Ok(fs.open(path, flags)?)
    }

    pub fn open_async(&self, flags: AccessRights) -> Result<XboxFile, LocalLocationError> {
        let (fs, path) = self.check_fs_cwd()?;

        Ok(fs.open_async(path, flags)?)
    }

    pub fn create(&self, flags: AccessRights) -> Result<XboxFile, LocalLocationError> {
        let (fs, path) = self.check_fs_cwd()?;

        Ok(fs.create(path, flags)?)
    }

    pub fn create_async(&self, flags: AccessRights) -> Result<XboxFile, LocalLocationError> {
        let (fs, path) = self.check_fs_cwd()?;

        Ok(fs.create_async(path, flags)?)
    }

    pub fn exists_directory(&self) -> Result<bool, LocalLocationError> {
        if self.is_read_only()? {
            return Ok(true);
        }

        Ok(match self.fs.as_ref().zip(self.cwd.as_ref()) {
            None => false,
            Some((fs, path)) => {
                let file_attr = match fs.fetch_attributes(path) {
                    Ok(x) => {x}
                    Err(_) => {
                        return Ok(false)
                    }
                };

                if file_attr & FILE_ATTRIBUTE_DIRECTORY != 0 {
                    true
                } else {
                    false
                }
            }
        })
    }

    pub fn exists_file(&self) -> Result<bool, LocalLocationError> {
        if self.is_read_only()? {
            return Ok(true);
        }

        Ok(match self.fs.as_ref().zip(self.cwd.as_ref()) {
            None => false,
            Some((fs, path)) => {
                let file_attr = match fs.fetch_attributes(path) {
                    Ok(x) => {x}
                    Err(_) => {
                        return Ok(false)
                    }
                };

                if file_attr & FILE_ATTRIBUTE_NORMAL != 0 {
                    true
                } else {
                    false
                }
            }
        })
    }

    pub fn exists(&self) -> Result<bool, LocalLocationError>{
        Ok(self.exists_file()? || self.exists_directory()?)
    }

    pub fn move_all(&self, to: &Self) -> Result<(), LocalLocationError>{
        if self.is_read_only()? || to.is_read_only()?{
            return Err(LocalLocationError::ReadOnly)
        }

        match self.cwd.as_ref().zip(to.cwd.as_ref()) {
            None => {},
            Some((from, to)) => {
                match self.fs {
                    None => {}
                    Some(fs) => {
                        fs.move_file(from, to)?;

                        return Ok(())
                    }
                }
            }
        };

        Err(LocalLocationError::MalformedPath)
    }

    // pub fn list_dir(&self) -> Result<Box<dyn Iterator<Item=XboxSearchResult>>, LocalLocationError> {
    //     if self.device.is_none() || self.fs.is_none() {
    //         return Ok(Box::new(DevicesSearch {
    //             device: self.device,
    //             state: 0,
    //         }))
    //     }
    //
    //     if self.cwd.is_none() {
    //         return Err(LocalLocationError::ReadOnly)
    //     }
    //
    //     Ok(Box::new(
    //         self.fs.as_ref().unwrap().list_dir(self.cwd.as_ref().unwrap())?
    //     ))
    // }

    pub fn list_dir(&self) -> Result<Box<dyn Iterator<Item = XboxSearchResult> + Send>, LocalLocationError> {
        match (&self.device, &self.fs, &self.cwd) {
            (Some(_), Some(fs), Some(cwd)) => {
                Ok(Box::new(fs.list_dir(cwd)?))
            }
            (_, Some(_), None) => {
                Err(LocalLocationError::ReadOnly)
            }
            _ => {
                Ok(Box::new(DevicesSearch {
                    device: self.device,
                    state: 0,
                }))
            }
        }
    }
}

pub struct DevicesSearch {
    device: Option<&'static XboxHardDisk>,
    state: usize
}

impl Iterator for DevicesSearch {
    type Item = XboxSearchResult;

    fn next(&mut self) -> Option<Self::Item> {
        match self.device.as_ref() {
            Some(device) => {
                if self.state > device.logical_mounts().len() - 1 {
                    return None;
                }

                let x = device.logical_mounts()[self.state];
                let res = XboxSearchResult::new(
                    XboxFileMetadata::new(
                        FileStandardInformation {
                            allocation_size: 0,
                            end_of_file: 0,
                            number_of_links: 0,
                            delete_pending: false,
                            directory: true
                        },
                        None
                    ),
                    Path::new(&format!("/{}/{}", &device.device_name(), x.mount_point())).ok()?
                );

                self.state += 1;

                Some(res)
            }
            None => {
                if self.state > INTERNAL_STORAGE.internal_devices().len() - 1 {
                    return None;
                }

                let x = INTERNAL_STORAGE.internal_devices()[self.state];
                let res = XboxSearchResult::new(
                    XboxFileMetadata::new(
                        FileStandardInformation {
                            allocation_size: 0,
                            end_of_file: 0,
                            number_of_links: 0,
                            delete_pending: false,
                            directory: true
                        },
                        None
                    ),
                    Path::new(&format!("/{}", &x.device_name())).ok()?
                );

                self.state += 1;

                Some(res)
            }
        }
    }
}

unsafe impl Send for DevicesSearch{}