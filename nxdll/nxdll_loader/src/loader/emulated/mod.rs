use crate::loader::runtime::loader::DLL_REGISTRY;
use crate::loader::runtime::registry::InMemoryDLL;
use alloc::sync::Arc;
use nxdll_bindings::modules::get_emulated_modules;

pub mod methods;

pub fn register_nx_emus() -> anyhow::Result<()> {
    let emu_modules = get_emulated_modules();

    {
        let mut registry = DLL_REGISTRY.lock();

        for module in emu_modules {
            let memory_dll = Arc::new(InMemoryDLL::new_emulated_boxed(module.0, module.1)?);
            registry.push(
                memory_dll
            );
        }
    }

    Ok(())
}
