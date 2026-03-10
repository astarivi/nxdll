use alloc::boxed::Box;
use core::cmp::min;
use nxdk_rs::sys::kernel::DbgPrint;

pub struct XboxLogger {
    level: log::LevelFilter
}

impl XboxLogger {
    pub fn new() -> Self {
        Self {
            level: log::LevelFilter::Info
        }
    }

    pub fn with_level(level: log::LevelFilter) -> Self {
        Self {
            level
        }
    }

    pub fn init(self) -> anyhow::Result<()> {
        log::set_max_level(self.level);
        log::set_logger(Box::leak(Box::new(self))).map_err(|e| anyhow::anyhow!(e))
    }

    fn get_cstr_no_alloc(msg: &str) -> [u8; 128] {
        let mut str_buffer: [u8; 128] = [0; 128];
        let msg_bytes = msg.as_bytes();
        let copy_amount = min(msg_bytes.len(), 127);

        str_buffer[0..copy_amount].copy_from_slice(&msg_bytes[..copy_amount]);

        if str_buffer[copy_amount - 1] != 0 {
            str_buffer[copy_amount] = 0;
        }

        str_buffer
    }
}

impl Default for XboxLogger {
    fn default() -> Self {
        XboxLogger::new()
    }
}

impl log::Log for XboxLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let message = format!("[{}] {}", record.level(), record.args());
        let c_str = Self::get_cstr_no_alloc(&message);

        unsafe {
            DbgPrint(c_str.as_ptr() as *const i8);
        }
    }

    fn flush(&self) {
    }
}

pub fn init_logger() -> anyhow::Result<()> {
    XboxLogger::default().init()
}