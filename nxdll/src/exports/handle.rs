use alloc::boxed::Box;
use core::ptr::null_mut;
use libc::c_char;
use crate::io::storage::location::Location;
use crate::loader::runtime::loader::register_from_disk;
use crate::loader::runtime::registry::{ArcMemoryDLL, PEDependency};
use crate::utils::cstr_ptr_to_str;

pub struct RegisteredDllHandle {
    pub inner: ArcMemoryDLL,
}

impl RegisteredDllHandle {
    pub fn new(inner: ArcMemoryDLL) -> Self {
        Self {
            inner
        }
    }

    pub fn boxed(inner: ArcMemoryDLL) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

pub struct LoadedDllHandle {
    pub inner: PEDependency,
}

impl LoadedDllHandle {
    pub fn new(inner: PEDependency) -> Self {
        Self {
            inner
        }
    }

    pub fn boxed(inner: PEDependency) -> Box<Self> {
        Box::new(Self::new(inner))
    }
}

#[no_mangle]
pub extern "C" fn nx_register_dll(c_path: *const c_char) -> *mut RegisteredDllHandle {
    let closure = | cp: *const c_char| -> anyhow::Result<*mut RegisteredDllHandle> {
        let path = cstr_ptr_to_str(cp)?;
        let win_location = Location::from_windows_path(path)?;

        let handle = RegisteredDllHandle::boxed(
            register_from_disk(&win_location)?.dll
        );

        Ok(Box::into_raw(handle))
    };

    closure(c_path)
        .inspect_err(|err| log::error!("register_dll: {}", err))
        .unwrap_or(null_mut())
}

#[no_mangle]
pub extern "C" fn nx_load_dll(handle: *mut RegisteredDllHandle) -> *mut LoadedDllHandle {
    let closure = | handle: *mut RegisteredDllHandle| -> anyhow::Result<*mut LoadedDllHandle> {
        let handle = unsafe { &*handle };
        let inner = &handle.inner;

        let dependency = inner.get_dependency(&handle.inner)?;

        Ok(Box::into_raw(LoadedDllHandle::boxed(dependency)))
    };

    closure(handle)
        .inspect_err(|err| log::error!("load_dll: {}", err))
        .unwrap_or(null_mut())
}

#[no_mangle]
pub extern "C" fn nx_unload_dll(handle: *mut LoadedDllHandle) {
    if handle.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(handle);
    }
}

#[no_mangle]
pub extern "C" fn nx_unregister_dll(handle: *mut RegisteredDllHandle) {
    if handle.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(handle);

    }
}

#[no_mangle]
pub extern "C" fn nx_get_func_by_ordinal(handle: *mut LoadedDllHandle, ordinal: u16) -> *const u8 {
    let closure = |handle: *mut LoadedDllHandle| -> anyhow::Result<*const u8> {
        let handle = unsafe { &*handle };
        let inner = &handle.inner;

        inner.dll.get_export_addr_by_ordinal(ordinal)
    };

    closure(handle)
        .inspect_err(|err| log::error!("get_func_by_ordinal: {}", err))
        .unwrap_or(null_mut())
}

#[no_mangle]
pub extern "C" fn nx_get_func_by_name(handle: *mut LoadedDllHandle, func_name: *const c_char) -> *const u8 {
    let closure = |handle: *mut LoadedDllHandle| -> anyhow::Result<*const u8> {
        let handle = unsafe { &*handle };
        let inner = &handle.inner;
        let func = cstr_ptr_to_str(func_name)?;

        inner.dll.get_export_addr_by_name(func)
    };

    closure(handle)
        .inspect_err(|err| log::error!("get_func_by_name: {}", err))
        .unwrap_or(null_mut())
}
