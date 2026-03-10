use alloc::boxed::Box;
use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;
use core::error::Error;
use core::fmt;
use core::str::Utf8Error;
use nxdk_rs::utils::error::PlatformError;

const MAX_FIXED_LEN: usize = 260;
const MAX_ALLOC_LEN: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PathError {
    PathTooLong,
    EmptyPath,
    RootPop,
    Utf8Error(Utf8Error),
    FromUtf8Error(FromUtf8Error),
    PlatformError(PlatformError),
}

impl From<Utf8Error> for PathError {
    fn from(e: Utf8Error) -> Self {
        PathError::Utf8Error(e)
    }
}

impl From<FromUtf8Error> for PathError {
    fn from(e: FromUtf8Error) -> Self {
        PathError::FromUtf8Error(e)
    }
}

impl From<PlatformError> for PathError {
    fn from(e: PlatformError) -> Self {
        PathError::PlatformError(e)
    }
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::PathTooLong => write!(f, "This path is too long, upper limit is {} chars",  MAX_ALLOC_LEN),
            PathError::RootPop => write!(f, "Tried to pop from a root path"),
            _ => {
                write!(f, "{:?}", self)
            }
        }
    }
}

impl Error for PathError {
}


/// Represents a POSIX path. All Paths created and returned with this type are expected to be
/// absolute.
#[derive(Clone, Eq, PartialEq)]
pub struct Path {
    path: PathInner,
}

#[derive(Eq, PartialEq)]
enum PathInner {
    Fixed([u8; MAX_FIXED_LEN]),
    Alloc(Box<[u8]>),
}

impl Clone for PathInner {
    fn clone(&self) -> Self {
        match self {
            PathInner::Fixed(array) => PathInner::Fixed(*array),
            PathInner::Alloc(boxed) => PathInner::Alloc(boxed.clone()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum PathType {
    Fixed,
    Alloc
}

impl Path {
    /// Create a new path from a given string
    pub fn new(path: &str) -> Result<Self, PathError> {
        // We allocate an extra + 1 for nul termination
        let mut normalized = Vec::with_capacity(path.len() + 1);

        for &byte in path.as_bytes() {
            match byte {
                b'\\' => normalized.push(b'/'),
                _ => normalized.push(byte)
            }
        }

        if normalized.len() > 1 && *normalized.last().unwrap() == b'/' {
            normalized.pop();
        }

        if normalized.is_empty() {
            return Err(PathError::EmptyPath);
        }

        // Null terminate the path
        normalized.push(0);

        Self::from_vec(normalized)
    }

    fn from_vec(path_bytes: Vec<u8>) -> Result<Self, PathError> {
        let path_len = path_bytes.len();

        if path_len <= MAX_FIXED_LEN {
            let mut buffer = [0u8; MAX_FIXED_LEN];
            buffer[..path_len].copy_from_slice(&path_bytes);
            Ok(Self {
                path: PathInner::Fixed(buffer),
            })
        } else if path_len <= MAX_ALLOC_LEN {
            Ok(Self {
                path: PathInner::Alloc(path_bytes.into_boxed_slice()),
            })
        } else {
            Err(PathError::PathTooLong)
        }
    }

    pub fn file_name(&self) -> anyhow::Result<Option<String>> {
        let len = self.len();
        let mut last_part = Vec::new();
        

        match &self.path {
            PathInner::Fixed(buffer) => {
                for i in (0..len).rev() {
                    let byte = buffer[i];
                    
                    if byte == b'/' {
                        break;
                    }

                    last_part.push(byte);
                }
            },
            PathInner::Alloc(boxed) => {
                for i in (0..len).rev() {
                    let byte = boxed[i];

                    if byte == b'/' {
                        break;
                    }

                    last_part.push(byte);
                }
            }
        }

        if last_part.is_empty() {
            Ok(None)
        } else {
            last_part.reverse();
            Ok(
                Some(
                    String::from_utf8(last_part).map_err(anyhow::Error::msg)?
                )
            )
        }
    }

    /// Check length
    pub fn len(&self) -> usize {
        match &self.path {
            PathInner::Fixed(buffer) => buffer
                .iter()
                .position(|&b| b == 0)
                .unwrap_or_else(|| panic!("Missing null terminator in Fixed path")),
            PathInner::Alloc(boxed) => boxed
                .iter()
                .position(|&b| b == 0)
                .unwrap_or_else(|| panic!("Missing null terminator in Alloc path")),
        }
    }

    /// Get the current path as bytes
    pub fn to_bytes(&self) -> &[u8] {
        let len = self.len();
        match &self.path {
            PathInner::Fixed(buffer) => &buffer[..len],
            PathInner::Alloc(boxed) => &boxed[..len],
        }
    }

    /// Add a file or folder to the path
    pub fn push(&self, file: &str) -> Result<Path, PathError> {
        let current_len = self.len();
        let file_bytes = file.as_bytes();

        // We add 2 extra to the path, 1 for nul termination, 1 for extra slash
        let mut normalized = Vec::with_capacity(current_len + file_bytes.len() + 2);

        match &self.path {
            PathInner::Fixed(buffer) => normalized.extend_from_slice(&buffer[..current_len]),
            PathInner::Alloc(boxed) => normalized.extend_from_slice(&boxed[..current_len])
        };

        if normalized.last().ok_or(PathError::EmptyPath)? != &b'/' {
            normalized.push(b'/');
        }

        for &byte in file_bytes {
            match byte {
                b'\\' => normalized.push(b'/'),
                _ => normalized.push(byte)
            }
        }

        normalized.push(0);

        Self::from_vec(normalized)
    }

    /// Go up one directory
    pub fn pop(&self) -> Result<Path, PathError> {
        let current_len = self.len();
        if current_len == 1 {
            return Err(PathError::RootPop);
        }

        let mut buffer = Vec::with_capacity(current_len);

        match &self.path {
            PathInner::Fixed(inner) => buffer.extend_from_slice(&inner[..current_len]),
            PathInner::Alloc(boxed) => buffer.extend_from_slice(&boxed[..current_len]),
        }

        if let Some(pos) = buffer.iter().rposition(|&b| b == b'/') {
            buffer.truncate(pos);

            if pos == 0 {
                buffer.push(b'/');
            }
        } else {
            buffer.clear();
            buffer.push(b'/');
        }

        buffer.push(0);

        Self::from_vec(buffer)
    }

    /// Convert the path to a Windows-style path
    pub fn to_windows(&self, mount_point: char) -> String {
        let current_len = self.len();
        let mut windows_path = String::with_capacity(current_len + 2);
        windows_path.push(mount_point);
        windows_path.push(':');

        // FIXME: This is ugly.
        match &self.path {
            PathInner::Fixed(buffer) => {
                for &byte in &buffer[..current_len] {
                    if byte == b'/' {
                        windows_path.push(b'\\' as char);
                    } else {
                        windows_path.push(byte as char);
                    }
                }
            },
            PathInner::Alloc(boxed) => {
                for &byte in &boxed[..current_len] {
                    if byte == b'/' {
                        windows_path.push(b'\\' as char);
                    } else {
                        windows_path.push(byte as char);
                    }
                }
            }
        }

        windows_path
    }

    /// Convert the path to an Xbox-style path, the destination buffer must be provided.
    /// Useful when looping through paths.
    pub fn write_xbox_path(&self, mount_point: &char, buffer: &mut [u8; 260]) -> Result<(), PathError> {
        let len = self.len();

        // Leave 3 spaces: mount letter + ":" + null terminator
        if len > 257 {
            return Err(PathError::PathTooLong);
        }

        buffer[0] = *mount_point as u8;
        buffer[1] = b':';

        let path = match &self.path {
            PathInner::Fixed(bf) => &bf[..len],
            PathInner::Alloc(boxed) => &boxed[..len],
        };

        let mut current_index = 2;

        for &byte in &path[..len] {
            if byte == b'/' {
                buffer[current_index] = b'\\';
            } else {
                buffer[current_index] = byte;
            }
            current_index += 1;
        }

        Ok(())
    }

    /// Convert the path to an Xbox-style path, allocates a new buffer for this path.
    pub fn to_xbox(&self, mount_point: &char) -> Result<[u8; 260], PathError> {
        let mut path_buffer: [u8; 260] = [0; 260];
        self.write_xbox_path(&mount_point, &mut path_buffer)?;
        Ok(path_buffer)
    }

    /// Get the current path as a string
    pub fn as_str(&self) -> Result<&str, PathError> {
        match &self.path {
            PathInner::Fixed(buffer) => {
                let len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
                Ok(core::str::from_utf8(&buffer[..len])?)
            }
            PathInner::Alloc(boxed) => Ok(core::str::from_utf8(&boxed)?),
        }
    }

    /// Convert the current path to a String
    pub fn to_string(&self) -> Result<String, PathError> {
        match &self.path {
            PathInner::Fixed(buffer) => {
                let len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
                Ok(String::from_utf8(buffer[..len].to_vec())?)
            }
            PathInner::Alloc(boxed) => {
                Ok(String::from_utf8(boxed.clone().to_vec())?)
            }
        }
    }

    pub fn inner_type(&self) -> PathType {
        match &self.path {
            PathInner::Fixed(_) => PathType::Fixed,
            PathInner::Alloc(_) => PathType::Alloc,
        }
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_str = self.as_str().unwrap_or("Invalid UTF-8 in path");
        write!(f, "Path({})", path_str)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_str = self.as_str().unwrap_or("Invalid UTF-8 in path");
        write!(f, "{}", path_str)
    }
}
