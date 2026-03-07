pub mod log;

use core::ffi::CStr;
use libc::c_char;
use nxdk_rs::utils::error::PlatformError;

pub fn cstr_ptr_to_str<'a>(c_path: *const c_char) -> Result<&'a str, PlatformError> {
    if c_path.is_null() {
        return Err(PlatformError::ReadError("c_path must not be null"));
    }

    let cstr = unsafe {
        CStr::from_ptr(c_path)
    };

    cstr.to_str().map_err(|_| PlatformError::ReadError("c_path is not valid UTF-8"))
}
