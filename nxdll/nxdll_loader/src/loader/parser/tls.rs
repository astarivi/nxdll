use crate::loader::parser::mapper::{LoadedImage, TLSInfo};
use crate::loader::parser::pe::ParsedPE;
use anyhow::{anyhow, bail};
use core::alloc::Layout;
use core::ptr;
use goblin::pe::tls::TLS_CHARACTERISTICS_ALIGN_MASK;
use log::error;
use nxdk_rs::sys::winapi::{FlsAlloc, FlsFree, FlsGetValue, FlsSetValue};
use crate::loader::parser::runtime::call_dll_main;

pub const DLL_PROCESS_DETACH: u32 = 0;
pub const DLL_PROCESS_ATTACH: u32 = 1;
pub const DLL_THREAD_ATTACH: u32 = 2;
pub const DLL_THREAD_DETACH: u32 = 3;

pub type TlsCallback = unsafe extern "system" fn(
    dll_handle: *mut core::ffi::c_void,
    reason: u32,
    reserved: *mut core::ffi::c_void,
);

fn va_to_ptr(image: &LoadedImage, va: usize) -> *mut u8 {
    let offset = va.checked_sub(image.preferred_base)
        .expect("VA below image base");

    unsafe { image.base.add(offset) }
}

fn tls_alignment(characteristics: u32) -> usize {
    const MASK: u32 = TLS_CHARACTERISTICS_ALIGN_MASK;

    let encoded = ((characteristics & MASK) >> 20) as usize;

    1usize << encoded.min(16)
}

/// Parse TLS
pub fn parse_tls(
    image: &mut LoadedImage,
    pe: &ParsedPE
) -> anyhow::Result<()> {

    let tls = pe.with_pe(|pe_raw| pe_raw.tls_data.clone());

    if let Some(tls_data) = tls {

        let dir = tls_data.image_tls_directory;

        let template_size = dir
            .end_address_of_raw_data
            .checked_sub(dir.start_address_of_raw_data)
            .ok_or_else(|| anyhow!("Invalid TLS template range"))? as usize;

        let template = if template_size != 0 {
            va_to_ptr(image, dir.start_address_of_raw_data as usize) as *const u8
        } else {
            ptr::null()
        };

        let callbacks = if dir.address_of_callbacks != 0 {
            va_to_ptr(image, dir.address_of_callbacks as usize)
                as *const *const TlsCallback
        } else {
            ptr::null()
        };

        let index_addr = if dir.address_of_index != 0 {
            va_to_ptr(image, dir.address_of_index as usize) as *mut u32
        } else {
            ptr::null_mut()
        };

        let zero_fill = dir.size_of_zero_fill as usize;

        let size = template_size
            .checked_add(zero_fill)
            .ok_or_else(|| anyhow!("TLS size overflow"))?
            .max(1);

        let align = tls_alignment(dir.characteristics).max(4).min(4096);
        let layout = Layout::from_size_align(size, align)
            .map_err(|_| anyhow!("Invalid TLS allocation layout"))?;

        image.tls_info = Some(TLSInfo {
            tls_index: 0,
            template,
            template_size,
            zero_fill,
            callbacks,
            index_addr,
            layout
        });
    }

    Ok(())
}

pub fn tls_init_process(image: &mut LoadedImage) -> anyhow::Result<()> {

    let tls = match &mut image.tls_info {
        Some(t) => t,
        None => return Ok(()),
    };

    unsafe {
        let index = FlsAlloc(None);

        if index == u32::MAX {
            bail!("FlsAlloc failed");
        }

        tls.tls_index = index;

        if !tls.index_addr.is_null() {
            *tls.index_addr = index;
        }
    }

    Ok(())
}

pub fn tls_init_thread(image: &LoadedImage) -> anyhow::Result<()> {

    let tls = match &image.tls_info {
        Some(t) => t,
        None => return Ok(()),
    };

    unsafe {

        let mem = alloc::alloc::alloc(tls.layout);
        if mem.is_null() {
            bail!("TLS allocation failed");
        }

        if !tls.template.is_null() && tls.template_size > 0 {
            ptr::copy_nonoverlapping(
                tls.template,
                mem,
                tls.template_size,
            );
        }

        if tls.zero_fill > 0 {
            ptr::write_bytes(
                mem.add(tls.template_size),
                0,
                tls.zero_fill,
            );
        }

        if FlsSetValue(tls.tls_index, mem as *mut _) == 0 {
            alloc::alloc::dealloc(mem, tls.layout);
            bail!("FlsSetValue failed");
        }
    }

    Ok(())
}

pub unsafe fn call_tls_callbacks(
    image: &LoadedImage,
    reason: u32,
) {
    let tls = match &image.tls_info {
        Some(t) => t,
        None => return,
    };

    if tls.callbacks.is_null() {
        return;
    }

    let mut cb = tls.callbacks;

    loop {
        let func = cb.read();

        if func.is_null() {
            break;
        }

        (*func)(
            image.base as *mut _,
            reason,
            ptr::null_mut(),
        );

        cb = cb.add(1);
    }
}

pub fn tls_deinit_thread(image: &LoadedImage) {
    let tls = match &image.tls_info {
        Some(t) => t,
        None => return,
    };

    match call_dll_main(image, DLL_THREAD_DETACH) {
        Ok(_) => {}
        Err(_) => {
            error!(
                    "Failed to call DLL main to deinit thread TLS with reason DLL_THREAD_DETACH"
                );
        }
    }

    unsafe {

        call_tls_callbacks(image, DLL_THREAD_DETACH);

        let mem = FlsGetValue(tls.tls_index);

        if !mem.is_null() {
            alloc::alloc::dealloc(mem as *mut u8, tls.layout);
            FlsSetValue(tls.tls_index, ptr::null_mut());
        }
    }
}

pub fn tls_deinit_process(image: &mut LoadedImage) {

    match call_dll_main(image, DLL_PROCESS_DETACH) {
        Ok(_) => {}
        Err(_) => {
            error!("Failed to call DLL main to deinit thread TLS with reason DLL_THREAD_DETACH");
        }
    }

    unsafe {
        call_tls_callbacks(image, DLL_PROCESS_DETACH);
    }

    let tls = match image.tls_info.as_mut() {
        Some(t) => t,
        None => return,
    };

    unsafe {
        FlsFree(tls.tls_index);
    }

    tls.tls_index = 0;
}
