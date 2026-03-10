use crate::loader::parser::mapper::LoadedImage;
use anyhow::bail;

pub type DllMain = unsafe extern "system" fn(
    hinst_dll: *mut core::ffi::c_void,
    reason: u32,
    reserved: *mut core::ffi::c_void,
) -> i32;

pub fn call_dll_main(
    image: &LoadedImage,
    reason: u32,
) -> anyhow::Result<()> {
    let entry = image.entry_point;

    if entry.is_null() {
        return Ok(());
    }

    let dll_main: DllMain = unsafe { core::mem::transmute(entry) };

    let result = unsafe {
        dll_main(
            image.base as *mut _,
            reason,
            core::ptr::null_mut(),
        )
    };

    if result == 0 {
        bail!("DllMain call failed, called with reason {}", reason);
    }

    Ok(())
}