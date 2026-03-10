use log::{log, Level};

pub extern "C" fn nx_log(ptr: *const u8, len: usize, log_type: u8) {
    let s = unsafe {
        let slice = core::slice::from_raw_parts(ptr, len);
        core::str::from_utf8_unchecked(slice)
    };

    let lvl = if log_type == 1 {
            Level::Trace
        } else if log_type == 1 {
            Level::Debug
        } else if log_type == 2 {
            Level::Info
        } else if log_type == 3 {
            Level::Warn
        } else {
            Level::Error
        };

    log!(lvl, "[DLL]: {}", s)
}