use crate::loader::runtime::registry::{ArcMemoryDLL, PEDependency};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use anyhow::{anyhow, Context};
use goblin::pe::PE;
use nxdk_rs::embedded_io::Read;
use nxdk_rs::winapi::file::AccessRights;
use nxdll_shared::io::storage::location::Location;
use nxdll_shared::io::INTERNAL_STORAGE;
use ouroboros::self_referencing;

#[self_referencing]
pub struct ParsedPE {
    pub bytes: Box<[u8]>,
    #[borrows(bytes)]
    #[not_covariant]
    pub pe: PE<'this>,
}

impl ParsedPE {
    pub fn create(location: &Location) -> anyhow::Result<ParsedPE> {
        let mut file = INTERNAL_STORAGE
            .hdd0()
            .fs_from_mount(location.mount)
            .context(format!("Failed to find {} partition. Not mounted?", location.mount))?
            .open(&location.path, AccessRights::Read)?;

        let mut result = vec!();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;

            if bytes_read == 0 {
                break;
            }

            result.extend_from_slice(&buffer[..bytes_read]);
        }

        file.close()?;

        // TODO: Check if PE is valid. (should be 32 bits and x86)
        Ok(ParsedPEBuilder {
            bytes: result.into_boxed_slice(),
            pe_builder: |bytes_ref| {
                PE::parse(bytes_ref).unwrap()
            }
        }.build())
    }

    /// Given the registry, will look for dependencies.
    /// Won't load DLL into memory or patch IAT.
    pub fn get_pe_dependencies(&self, registry: &Vec<ArcMemoryDLL>) -> anyhow::Result<Vec<PEDependency>> {
        let (imports, own_name) = self.with_pe(|pe_raw|
            (&pe_raw.imports, pe_raw.name.unwrap_or("unknown").to_string())
        );

        let mut deps: Vec<PEDependency> = Vec::new();

        for import in imports {
            let dll = {
                registry
                    .iter()
                    .find(|d| d.get_name().eq_ignore_ascii_case(import.dll))
                    .cloned()
            }.ok_or_else(|| anyhow!(
                "Required DLL {} referenced by DLL {} not found (Registry time)",
                import.dll,
                own_name,
            ))?;

            if dll.is_emulated() {
                continue;
            }

            if !deps.iter().any(|d| Arc::ptr_eq(&d.dll, &dll)) {
                deps.push(dll.get_dependency(&dll)?);
            }
        }

        Ok(deps)
    }

    pub fn unload(self) {
        let _pe = self.into_heads().bytes;
    }
}
