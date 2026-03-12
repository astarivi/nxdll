use crate::loader::parser::exports::build_exports;
use crate::loader::parser::pe::ParsedPE;
use crate::loader::parser::runtime::call_dll_main;
use crate::loader::parser::{mapper, tls};
use crate::loader::runtime::registry::{ArcMemoryDLL, InMemoryDLL, PEExportedFunction, PEMappedImage};
use alloc::sync::Arc;
use alloc::vec::Vec;
use anyhow::{anyhow, bail};
use lazy_static::lazy_static;
use nxdll_shared::io::storage::location::Location;
use nxdll_shared::io::threading::mutex::Mutex;

lazy_static! {
    pub static ref DLL_REGISTRY: Mutex<Vec<ArcMemoryDLL>> = Mutex::new(Vec::new());
}

pub fn load_from_disk(path: &Location) -> anyhow::Result<PEMappedImage> {
    let registry = DLL_REGISTRY.lock();

    let parsed_pe = ParsedPE::create(path)?;
    let mut loaded_image = mapper::LoadedImage::new(&parsed_pe)?;

    unsafe {
        mapper::copy_headers(&loaded_image, &parsed_pe)?;
        mapper::load_sections(&loaded_image, &parsed_pe)?;
        mapper::perform_relocations(&loaded_image, &parsed_pe)?;
    }

    mapper::resolve_imports(
        &loaded_image,
        &parsed_pe,
        &registry,
    )?;

    let exports = build_exports(&loaded_image, &parsed_pe)?;

    tls::parse_tls(&mut loaded_image, &parsed_pe)?;
    tls::tls_init_process(&mut loaded_image)?;
    tls::tls_init_thread(&mut loaded_image)?;

    unsafe {
        tls::call_tls_callbacks(&mut loaded_image, tls::DLL_PROCESS_ATTACH);
    }

    call_dll_main(&loaded_image, tls::DLL_PROCESS_ATTACH)?;

    Ok(PEMappedImage::new(loaded_image, exports))
}

/// Adds a given DLL to the registry.
pub fn register_from_disk(path: &Location) -> anyhow::Result<ArcMemoryDLL> {
    let dll_name = path.path.file_name()?
        .ok_or_else(|| anyhow!("No filename found in path"))?;

    let mut registry = DLL_REGISTRY.lock();

    if registry
        .iter()
        .find(|x| x.get_name() == &dll_name)
        .is_some()
    {
        bail!("This DLL is already registered");
    }

    let parsed_pe = ParsedPE::create(path)?;
    let pe_deps = parsed_pe.get_pe_dependencies(&registry)?;

    let memory_dll = Arc::new(InMemoryDLL::new_real(
        path,
        pe_deps
    )?);

    registry.push(
        Arc::clone(&memory_dll)
    );

    Ok(memory_dll)
}

/// Adds emulated DLL to the registry.
pub fn register_emulated(dll_name: &str, exports: Vec<PEExportedFunction>) -> anyhow::Result<ArcMemoryDLL>{
    let mut registry = DLL_REGISTRY.lock();
    if registry
        .iter()
        .find(|x| x.get_name() == dll_name)
        .is_some()
    {
        bail!("This DLL is already registered");
    }

    let memory_dll = Arc::new(InMemoryDLL::new_emulated(dll_name, exports)?);

    registry.push(
        Arc::clone(&memory_dll)
    );

    Ok(memory_dll)
}
