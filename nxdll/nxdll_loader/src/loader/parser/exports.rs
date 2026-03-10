use alloc::vec::Vec;
use crate::loader::parser::mapper::LoadedImage;
use crate::loader::parser::pe::ParsedPE;
use crate::loader::runtime::registry::PEExportedFunction;

pub fn build_exports(
    image: &LoadedImage,
    pe: &ParsedPE,
) -> anyhow::Result<Vec<PEExportedFunction>> {

    let mut out = Vec::new();

    pe.with_pe(|pe_raw| {
        for export in &pe_raw.exports {
            // TODO: Support reexports.
            if export.reexport.is_some() {
                continue;
            }

            let name = match export.name {
                Some(n) => Some(n.into()),
                None => {None},
            };

            let ordinal = match export.offset {
                Some(o) => o as u16,
                None => continue,
            };

            let addr = unsafe {
                image.base.add(export.rva)
            };

            out.push(PEExportedFunction {
                name,
                ordinal,
                addr,
            });
        }
    });

    Ok(out)
}