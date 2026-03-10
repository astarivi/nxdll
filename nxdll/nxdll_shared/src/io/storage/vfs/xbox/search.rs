use alloc::boxed::Box;
use log::error;
use nxdk_rs::sys::winapi::{FindClose, FindFirstFileA, FindNextFileA, ERROR_FILE_NOT_FOUND, FILE_ATTRIBUTE_DIRECTORY, HANDLE, WIN32_FIND_DATAA};
use nxdk_rs::winapi::error::WinError;
use nxdk_rs::winapi::file::FileStandardInformation;
use crate::io::storage::path::{Path, PathError};
use crate::io::storage::vfs::error::VFileSystemError;
use crate::io::storage::vfs::xbox::file::{XboxExtendedFileMetadata, XboxFileMetadata};
use crate::io::storage::vfs::xbox::INVALID_HANDLE_VALUE;
use crate::io::storage::vfs::xbox::utils::date_from_lohi;

impl SearchHandle {
    pub fn list_dir(path: &Path, mount_point: &char) -> Result<Box<dyn Iterator<Item=XboxSearchResult> + Send>, VFileSystemError> {
        // Leave 5 spaces: mount letter + ":" + "\" + "*" + null terminator
        if path.len() > 255 {
            return Err(VFileSystemError::from(PathError::PathTooLong));
        }

        let mut has_next_file = true;
        let mut xbox_path = path.to_xbox(mount_point)?;

        let mut null_pos = xbox_path.iter().position(|&b| b == 0).unwrap_or_else(|| panic!("Couldn't find nul terminator on nul terminated"));
        null_pos = if xbox_path[null_pos - 1] == b'\\' {
            null_pos - 1
        } else {
            null_pos
        };
        
        xbox_path[null_pos] = b'\\';
        xbox_path[null_pos + 1] = b'*';
        xbox_path[null_pos + 2] = 0;

        let mut find_data: WIN32_FIND_DATAA = unsafe { core::mem::zeroed() };
        let search_handle: HANDLE = unsafe {
            FindFirstFileA(xbox_path.as_ptr() as *const i8, &mut find_data)
        };

        if search_handle == INVALID_HANDLE_VALUE {
            let last_error = WinError::from_last_error();

            // Empty folder
            if last_error.into_inner() == ERROR_FILE_NOT_FOUND {
                has_next_file = false;
            } else {
                return Err(last_error.into());
            }
        }

        Ok(Box::new(Self {
            handle: Some(search_handle),
            find_data,
            root_path: path.clone(),
            has_next_file
        }))
    }

    fn next(&mut self) -> Result<Option<XboxSearchResult>, VFileSystemError> {
        if !self.has_next_file || self.is_closed() {
            self.close();
            return Ok(None);
        }

        let mut found_nul = false;
        let rust_chars: [u8; 260] = self.find_data.cFileName.map(|b| {
            if found_nul {
                0
            } else if b == 0 {
                found_nul = true;
                0
            } else {
                b as u8
            }
        });

        let creation_time = if self.find_data.ftCreationTime.dwHighDateTime != 0 && self.find_data.ftCreationTime.dwLowDateTime != 0 {
            Some(date_from_lohi(self.find_data.ftCreationTime.dwLowDateTime,  self.find_data.ftCreationTime.dwHighDateTime)?)
        } else {
            None
        };

        let modification_time = date_from_lohi(self.find_data.ftLastWriteTime.dwLowDateTime,  self.find_data.ftLastWriteTime.dwHighDateTime)?;

        let size = ((self.find_data.nFileSizeHigh as u64) << 32) | (self.find_data.nFileSizeLow as u64);
        
        let result = XboxSearchResult {
            metadata: XboxFileMetadata::new(
                FileStandardInformation {
                    allocation_size: 0,
                    end_of_file: size,
                    number_of_links: 0,
                    delete_pending: false,
                    directory: self.find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0
                },
                Some(XboxExtendedFileMetadata::new(
                    creation_time,
                    modification_time
                ))
            ),
            path: self.root_path.push(
                core::str::from_utf8(&rust_chars).map_err(|e| PathError::from(e))?
            )?
        };

        let next_file = unsafe {FindNextFileA(self.handle.ok_or(VFileSystemError::ClosedHandle)?, &mut self.find_data)};

        if next_file == 0 {
            self.has_next_file = false;
            self.close();
        }

        Ok(Some(result))
    }

    pub fn is_closed(&self) -> bool {
        self.handle.is_none()
    }

    pub fn close(&mut self) {
        if let Some(handle) = self.handle.take() {
            unsafe {
                // FIXME: Check if this actually closed
                FindClose(handle);
            }
        }
    }
}

pub struct XboxSearchResult {
    metadata: XboxFileMetadata,
    path: Path
}

impl XboxSearchResult {
    pub fn new(metadata: XboxFileMetadata, path: Path) -> Self {
        Self {
            metadata,
            path
        }
    }

    pub fn metadata(&self) -> &XboxFileMetadata {
        &self.metadata
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct SearchHandle {
    handle: Option<HANDLE>,
    find_data: WIN32_FIND_DATAA,
    root_path: Path,
    has_next_file: bool,
}

unsafe impl Send for SearchHandle {}

impl Iterator for SearchHandle {
    type Item = XboxSearchResult;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next() {
            Ok(Some(result)) => Some(result),
            Ok(None) => None,
            Err(err) => {
                error!("Error getting next file: {:?}", err);
                self.close();
                None
            }
        }
    }
}