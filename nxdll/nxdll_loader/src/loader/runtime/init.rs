use nxdll_shared::utils::log::init_logger;
use crate::loader::emulated::register_nx_emus;

pub fn loader_init() -> anyhow::Result<()> {
    let _ = init_logger()?;

    register_nx_emus()?;

    Ok(())
}