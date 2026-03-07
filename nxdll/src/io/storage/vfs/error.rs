use crate::io::storage::path::PathError;
use core::error::Error;
use core::fmt::{Debug, Display, Formatter};
use nxdk_rs::embedded_io::ErrorKind;
use nxdk_rs::winapi::error::{WinError, WinMixedError};
use time::error::ComponentRange;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VFileError {
    WinPlatformError(WinMixedError)
}

impl From<WinMixedError> for VFileError {
    fn from(err: WinMixedError) -> VFileError {
        VFileError::WinPlatformError(err)
    }
}

impl Display for VFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for VFileError {}

impl nxdk_rs::embedded_io::Error for VFileError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VFileSystemError {
    PathError(PathError),
    FileDateOutOfBounds(ComponentRange),
    ClosedHandle,
    WinPlatformError(WinMixedError)
}

impl From<PathError> for VFileSystemError {
    fn from(err: PathError) -> VFileSystemError {
        Self::PathError(err)
    }
}

impl From<ComponentRange> for VFileSystemError {
    fn from(err: ComponentRange) -> VFileSystemError {
        Self::FileDateOutOfBounds(err)
    }
}

impl From<WinMixedError> for VFileSystemError {
    fn from(err: WinMixedError) -> VFileSystemError {
        Self::WinPlatformError(err)
    }
}

impl From<WinError> for VFileSystemError {
    fn from(err: WinError) -> VFileSystemError {
        VFileSystemError::WinPlatformError(err.into())
    }
}

impl Display for VFileSystemError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for VFileSystemError {}