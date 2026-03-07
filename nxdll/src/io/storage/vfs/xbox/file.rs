use nxdk_rs::winapi::error::WinMixedError;
use nxdk_rs::winapi::file::{FileStandardInformation, WinFileHandle};
use time::OffsetDateTime;
use crate::io::storage::path::Path;
use crate::io::storage::vfs::error::VFileError;
use nxdk_rs::embedded_io as embedded_io;
use nxdk_rs::embedded_io_async as embedded_io_async;

pub struct XboxFile {
    handle: WinFileHandle,
    path: Path
}

unsafe impl Send for XboxFile {}

impl embedded_io::ErrorType for XboxFile {
    type Error = VFileError;
}

impl embedded_io::Read for XboxFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(
            embedded_io::Read::read(&mut self.handle, buf).map_err(WinMixedError::from)?
        )
    }
}

impl embedded_io::Write for XboxFile {

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(
            embedded_io::Write::write(&mut self.handle, buf).map_err(WinMixedError::from)?
        )
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(
            embedded_io::Write::flush(&mut self.handle).map_err(WinMixedError::from)?
        )
    }
}

impl embedded_io::Seek for XboxFile {
    fn seek(&mut self, pos: embedded_io::SeekFrom) -> Result<u64, Self::Error> {
        Ok(
            embedded_io::Seek::seek(&mut self.handle, pos).map_err(WinMixedError::from)?
        )
    }
}

impl embedded_io_async::Read for XboxFile {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(
            embedded_io_async::Read::read(&mut self.handle, buf).await.map_err(WinMixedError::from)?
        )
    }
}

impl embedded_io_async::Write for XboxFile {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(
            embedded_io_async::Write::write(&mut self.handle, buf).await.map_err(WinMixedError::from)?
        )
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(
            embedded_io_async::Write::flush(&mut self.handle).await.map_err(WinMixedError::from)?
        )
    }
}

impl embedded_io_async::Seek for XboxFile {
    async fn seek(&mut self, pos: embedded_io::SeekFrom) -> Result<u64, Self::Error> {
        Ok(
            embedded_io_async::Seek::seek(&mut self.handle, pos).await.map_err(WinMixedError::from)?
        )
    }
}

impl XboxFile {
    pub fn new(handle: WinFileHandle, path: Path) -> Self {
        Self {
            handle,
            path
        }
    }

    pub fn metadata(&self) -> Result<XboxFileMetadata, VFileError> {
        let query = self.handle.query_standard_information()?;

        Ok(XboxFileMetadata {
            basic: query,
            extended: None,
        })
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn close(&mut self) -> Result<(), VFileError> {
        Ok(self.handle.close().map_err(WinMixedError::from)?)
    }
}

pub struct XboxExtendedFileMetadata {
    creation_time: Option<OffsetDateTime>,
    modification_time: OffsetDateTime
}

impl XboxExtendedFileMetadata {
    pub fn new(creation_time: Option<OffsetDateTime>, modification_time: OffsetDateTime) -> Self {
        Self {
            creation_time,
            modification_time
        }
    }
}

pub struct XboxFileMetadata {
    basic: FileStandardInformation,
    extended: Option<XboxExtendedFileMetadata>
}

impl XboxFileMetadata {
    pub fn new(basic: FileStandardInformation, extended: Option<XboxExtendedFileMetadata>) -> Self {
        Self {
            basic,
            extended
        }
    }

    pub fn basic(&self) -> &FileStandardInformation {
        &self.basic
    }

    pub fn extended(&self) -> Option<&XboxExtendedFileMetadata> {
        self.extended.as_ref()
    }

    pub fn is_dir(&self) -> bool {
        self.basic.directory
    }

    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }

    pub fn is_symlink(&self) -> Option<bool> {
        None
    }

    pub fn len(&self) -> u64 {
        self.basic.end_of_file
    }

    pub fn modified(&self) -> Option<&OffsetDateTime> {
        Some(&self.extended.as_ref()?.modification_time)
    }

    pub fn created(&self) -> Option<&OffsetDateTime> {
        self.extended.as_ref()?.creation_time.as_ref()
    }
}
