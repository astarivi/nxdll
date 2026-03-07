use core::error::Error;
use core::fmt;
use nxdk_rs::winapi::error::{WinError, WinMixedError};
use time::error::ComponentRange;
use crate::io::storage::path::PathError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum XboxFsError {
    PathError(PathError),
    FileDateOutOfBounds(ComponentRange),
    ClosedHandle,
    WinError(WinMixedError)
}

impl From<WinMixedError> for XboxFsError {
    fn from(err: WinMixedError) -> Self {
        Self::WinError(err)
    }
}

impl From<WinError> for XboxFsError {
    fn from(err: WinError) -> Self {
        Self::WinError(
            WinMixedError::WinError(err)
        )
    }
}

impl From<PathError> for XboxFsError {
    fn from(err: PathError) -> Self {
        Self::PathError(err)
    }
}

impl From<ComponentRange> for XboxFsError {
    fn from(err: ComponentRange) -> Self {
        Self::FileDateOutOfBounds(err)
    }
}

impl fmt::Display for XboxFsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for XboxFsError {
}