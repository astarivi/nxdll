use crate::loader::parser::pe::ParsedPE;
use crate::loader::parser::tls;
use crate::loader::parser::tls::TlsCallback;
use crate::loader::runtime::registry::{ArcMemoryDLL, PEDependency};
use alloc::sync::Arc;
use alloc::vec::Vec;
use anyhow::{anyhow, bail, Context};
use core::alloc::Layout;
use core::ptr;
use goblin::pe::section_table::IMAGE_SCN_CNT_UNINITIALIZED_DATA;
use goblin::pe::PE;
use nxdk_rs::sys::winapi::*;

pub struct TLSInfo {
    /// TLS slot index used with FLS/TLS APIs
    pub tls_index: u32,

    /// Pointer to TLS initialization template
    pub template: *const u8,

    /// Size of the initialization template
    pub template_size: usize,

    /// Extra zeroed bytes required by the TLS block
    pub zero_fill: usize,

    /// Pointer to the TLS callbacks array
    pub callbacks: *const *const TlsCallback,

    /// Pointer inside the module where the TLS index must be written
    pub index_addr: *mut u32,

    pub layout: Layout
}

/// Holds the PE image in memory.
pub struct LoadedImage {
    pub base: *mut u8,
    pub size: u32,
    pub preferred_base: usize,
    pub entry_point: *mut u8,
    pub tls_info: Option<TLSInfo>,
}

impl Drop for LoadedImage {
    fn drop(&mut self) {
        tls::tls_deinit_thread(self);
        tls::tls_deinit_process(self);

        unsafe {
            VirtualFree(self.base as *mut _, 0, MEM_RELEASE);
        }
    }
}

#[repr(C)]
pub struct IMAGE_BASE_RELOCATION {
    pub VirtualAddress: u32,
    pub SizeOfBlock: u32,
}

impl LoadedImage {
    /// Allocates memory to load PE image into memory. No copy just yet.
    pub fn new(pe: &ParsedPE) -> anyhow::Result<Self> {
        let mut optional_header = None;
        let mut image_base = 0;

        pe.with_pe(|pe_raw: &PE| {
            optional_header = pe_raw.header.optional_header.as_ref();
            image_base = pe_raw.image_base as usize;
        });

        let size = optional_header
            .context("Optional header not found; needed to load image")?
            .windows_fields
            .size_of_image as u32;

        unsafe {
            let base = VirtualAlloc(
                image_base as *mut _,
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            );

            let base = if base.is_null() {
                VirtualAlloc(
                    ptr::null_mut(),
                    size,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_EXECUTE_READWRITE,
                )
            } else {
                base
            };

            if base.is_null() {
                bail!("Failed to allocate memory for image");
            }

            let entry: u32 = pe.with_pe(|pe_raw: &PE| {
                pe_raw.entry
            });

            let entry_addr = unsafe {
                (base as *mut u8).add(entry as usize)
            };

            Ok(LoadedImage {
                base: base as *mut u8,
                size,
                preferred_base: image_base,
                tls_info: None,
                entry_point: entry_addr
            })
        }
    }
}

/// Copies PE headers to allocated memory.
pub unsafe fn copy_headers(
    image: &LoadedImage,
    pe: &ParsedPE,
) -> anyhow::Result<()> {
    let mut optional_header = None;

    pe.with_pe(|pe_raw: &PE| {
        optional_header = pe_raw.header.optional_header.as_ref();
    });

    let headers_size = optional_header
        .context("Optional header not found; needed to load image")?
        .windows_fields
        .size_of_headers as usize;

    let dll_bytes = pe.borrow_bytes();

    core::ptr::copy_nonoverlapping(
        dll_bytes.as_ptr(),
        image.base,
        headers_size,
    );

    Ok(())
}

/// Load sections into memory.
pub unsafe fn load_sections(
    image: &LoadedImage,
    pe: &ParsedPE,
) -> anyhow::Result<()> {

    let bytes = pe.borrow_bytes();

    pe.with_pe(|pe_raw: &PE| {
        for section in &pe_raw.sections {
            let dest = image.base.add(section.virtual_address as usize);

            let raw_size = section.size_of_raw_data as usize;
            let virt_size = section.virtual_size as usize;

            let raw_ptr = section.pointer_to_raw_data as usize;

            // Copy section data
            if raw_size > 0 {
                let src = bytes.as_ptr().add(raw_ptr);

                ptr::copy_nonoverlapping(
                    src,
                    dest,
                    raw_size,
                );
            }

            // Zero remainder (BSS tail)
            if virt_size > raw_size {
                ptr::write_bytes(
                    dest.add(raw_size),
                    0,
                    virt_size - raw_size,
                );
            }

            // Full BSS section
            if section.characteristics & IMAGE_SCN_CNT_UNINITIALIZED_DATA != 0 {
                ptr::write_bytes(
                    dest,
                    0,
                    virt_size,
                );
            }
        }

    });

    Ok(())
}

/// Performs memory relocations. Used when DLL didn't load in the preferred base.
pub unsafe fn perform_relocations(
    image: &LoadedImage,
    pe: &ParsedPE,
) -> anyhow::Result<()> {

    pe.with_pe(|pe_raw: &PE| {

        let preferred_base = image.preferred_base as isize;
        let actual_base = image.base as isize;

        let delta = actual_base - preferred_base;

        if delta == 0 {
            return;
        }

        let reloc_dir = match pe_raw
            .header
            .optional_header
            .as_ref()
            .and_then(|opt| opt.data_directories.get_base_relocation_table())
        {
            Some(dir) if dir.size > 0 => dir,
            _ => return,
        };

        let reloc_start = image.base.add(reloc_dir.virtual_address as usize);
        let reloc_end = reloc_start.add(reloc_dir.size as usize);

        let mut block = reloc_start as *mut IMAGE_BASE_RELOCATION;

        while (block as *mut u8) < reloc_end {

            let page_rva = (*block).VirtualAddress;
            let block_size = (*block).SizeOfBlock;

            let entry_count =
                (block_size as usize - size_of::<IMAGE_BASE_RELOCATION>())
                    / 2;

            let entries =
                (block as *mut u8).add(size_of::<IMAGE_BASE_RELOCATION>())
                    as *mut u16;

            for i in 0..entry_count {

                let entry = *entries.add(i);

                let typ = entry >> 12;
                let offset = entry & 0x0fff;

                let patch_addr = image
                    .base
                    .add(page_rva as usize + offset as usize)
                    as *mut u32;

                match typ {
                    IMAGE_REL_BASED_HIGHLOW => {
                        let val = ptr::read(patch_addr);
                        ptr::write(patch_addr, (val as isize + delta) as u32);
                    }

                    _ => {
                        // unsupported relocation
                    }
                }
            }

            block = (block as *mut u8)
                .add(block_size as usize)
                as *mut IMAGE_BASE_RELOCATION;
        }
    });

    Ok(())
}

/// Patch the IAT of a loaded image. This will allow DLL -> Host or DLL -> DLL
/// communication
pub fn resolve_imports(
    image: &LoadedImage,
    pe: &ParsedPE,
    registry: &Vec<ArcMemoryDLL>
) -> anyhow::Result<Vec<PEDependency>> {
    let imports = pe.with_pe(|pe_raw| &pe_raw.imports);

    let mut deps: Vec<PEDependency> = Vec::new();

    for import in imports {
        let dll = {
            registry
                .iter()
                .find(|d| d.get_name().eq_ignore_ascii_case(import.dll))
                .cloned()
        }.ok_or_else(|| anyhow!(
            "Required DLL {} for IAT not found",
            import.dll
        ))?;

        if !deps.iter().any(|d| Arc::ptr_eq(&d.dll, &dll)) {
            deps.push(dll.get_dependency(&dll)?);
        }

        let export_addr = if import.ordinal != 0 {
            dll.get_export_addr_by_ordinal(import.ordinal)?
        } else {
            dll.get_export_addr_by_name(&import.name)?
        };

        unsafe {
            let iat_entry = image.base.add(import.offset) as *mut u32;
            ptr::write(iat_entry, export_addr as u32);
        }
    }

    Ok(deps)
}
