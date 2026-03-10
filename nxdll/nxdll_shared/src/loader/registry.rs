use alloc::boxed::Box;

pub struct PEExportedFunction {
    pub name: Option<Box<str>>,
    pub ordinal: u16,
    pub addr: *const u8,
}
