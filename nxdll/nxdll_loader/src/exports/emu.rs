use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::CStr;
use core::ptr::null_mut;
use anyhow::bail;
use libc::c_char;
use nxdll_shared::utils::cstr_ptr_to_str;
use crate::exports::handle::RegisteredDllHandle;
use crate::loader::runtime::loader::{register_emulated, register_from_disk};
use crate::loader::runtime::registry::PEExportedFunction;

#[repr(C)]
pub struct C_PEExportedFunction {
    pub name: *const c_char,
    pub ordinal: u16,
    pub addr: *const u8,
}

#[repr(C)]
pub struct C_EmulatedDLL {
    pub functions: *const C_PEExportedFunction,
    pub num_functions: usize,
}

impl C_EmulatedDLL {
    pub fn to_rust(&self) -> Vec<PEExportedFunction> {
        let mut exports = Vec::with_capacity(self.num_functions);

        unsafe {
            let funcs = core::slice::from_raw_parts(self.functions, self.num_functions);
            for f in funcs {
                let name = if !f.name.is_null() {
                    Some(CStr::from_ptr(f.name).to_string_lossy().into_owned().into_boxed_str())
                } else {
                    None
                };

                exports.push(PEExportedFunction {
                    name,
                    ordinal: f.ordinal,
                    addr: f.addr,
                });
            }
        }

        exports
    }
}

#[no_mangle]
pub extern "C" fn nx_register_emulated_dll(dll_name: *const c_char, dll: *const C_EmulatedDLL) -> *mut RegisteredDllHandle {
    let closure = || -> anyhow::Result<*mut RegisteredDllHandle> {
        let name = cstr_ptr_to_str(dll_name)?;

        if dll.is_null() {
            bail!("Exports is null");
        }

        let dll_ref = unsafe { &*dll };
        let exports = dll_ref.to_rust();

        let handle = RegisteredDllHandle::boxed(
            register_emulated(name, exports)?.dll
        );

        Ok(Box::into_raw(handle))
    };

    closure()
        .inspect_err(|err| log::error!("nx_register_emulated_dll: {}", err))
        .unwrap_or(null_mut())
}
