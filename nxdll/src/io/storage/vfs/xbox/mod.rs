pub mod fs;
pub mod error;
pub mod search;
pub mod utils;
pub mod file;

pub const INVALID_HANDLE_VALUE: *mut core::ffi::c_void = -1isize as *mut core::ffi::c_void;
pub const MAX_PATH: u32 = 260;