use alloc::boxed::Box;
use anyhow::Context;
use goblin::pe::PE;
use nxdk_rs::embedded_io::Read;
use nxdk_rs::winapi::file::AccessRights;
use ouroboros::self_referencing;
use nxdll_shared::io::INTERNAL_STORAGE;
use nxdll_shared::io::storage::location::Location;

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

    pub fn unload(self) {
        let _pe = self.into_heads().bytes;
    }
}
