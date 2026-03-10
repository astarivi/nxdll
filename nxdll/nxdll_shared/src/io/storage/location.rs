use alloc::string::String;
use crate::io::storage::path::{Path, PathError};

/// Represents a storage location with a Windows-style mount point and a path
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Location {
    pub mount: char,
    pub path: Path,
}

impl Location {
    /// Create a StorageLocation from a Windows path, e.g., "C:\folder\file.txt"
    pub fn from_windows_path(win_path: &str) -> Result<Self, PathError> {
        let mut chars = win_path.chars();

        // Extract mount point
        let mount = chars.next().ok_or(PathError::EmptyPath)?;
        let colon = chars.next().ok_or(PathError::EmptyPath)?;
        if colon != ':' {
            return Err(PathError::EmptyPath);
        }

        // The rest is the path
        let remaining: String = chars.collect();
        let path = Path::new(&remaining)?;

        Ok(Self { mount, path })
    }

    /// Convert back to a Windows-style path
    pub fn to_windows_string(&self) -> String {
        self.path.to_windows(self.mount)
    }

    /// Convert to a Unix-style path string (without mount)
    pub fn to_unix_string(&self) -> Result<String, PathError> {
        self.path.to_string()
    }

    /// Convenience: push a file or folder
    pub fn push(&self, file: &str) -> Result<Self, PathError> {
        Ok(Self {
            mount: self.mount,
            path: self.path.push(file)?,
        })
    }

    /// Convenience: pop one level
    pub fn pop(&self) -> Result<Self, PathError> {
        Ok(Self {
            mount: self.mount,
            path: self.path.pop()?,
        })
    }
}