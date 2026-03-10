use crate::loader::parser::mapper::LoadedImage;
use crate::loader::runtime::loader::load_from_disk;
use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use anyhow::anyhow;
use nxdll_shared::io::storage::location::Location;
use nxdll_shared::io::threading::mutex::Mutex;
pub use nxdll_shared::loader::registry::PEExportedFunction as PEExportedFunction;

pub type ArcMemoryDLL = Arc<InMemoryDLL>;

/// Holds a reference to both the InMemoryDLL and PEMappedImage.
/// Why?, PEMappedImage should unload if it has no active users,
/// and the registry itself (DLL_REGISTRY) is a Weak reference,
/// so it's not an active user.
///
/// This means that even if we have >1 reference to ArcMemoryDLL,
/// PEMappedImage will unload, causing UB when the consumer invokes
/// a function pointer from PEExportedFunction.
///
/// Why keep two reference counts?
///
/// 1. ArcMemoryDLL reference count is meant to signal the
/// DLL is registered / known by us.
///
/// 2. PEMappedImage signals the DLL is available in memory.
pub struct PEDependency {
    pub dll: ArcMemoryDLL,

    /// None if emulated. Keeps the PEMappedImage alive.
    pub image: Option<Arc<PEMappedImage>>
}

pub struct InMemoryDLL {
    /// Name of the DLL, including extension. Ex: "libcurl.dll"
    name: Box<str>,

    /// Actual PE structure in memory.
    container: MemoryContainer,

    /// Keep runtime dependencies.
    dependencies: Vec<PEDependency>,
}

pub enum MemoryContainer {
    Emulated(EmuContainer),
    Loaded(PEContainer),
}

pub struct EmuContainer {
    emulated_exports: Vec<PEExportedFunction>
}

pub struct PEContainer {
    path: Location,
    image: Mutex<Weak<PEMappedImage>>,
}

pub struct PEMappedImage {
    loaded_image: LoadedImage,
    exports: Vec<PEExportedFunction>,
}

// pub struct PEExportedFunction {
//     pub name: Option<Box<str>>,
//     pub ordinal: u16,
//     pub addr: *const u8,
// }

impl InMemoryDLL {
    pub fn new_real(path: &Location, mpd_image: PEMappedImage, dependencies: Vec<PEDependency>) -> anyhow::Result<Self> {
        let arc_img = Arc::new(mpd_image);

        Ok(Self {
            name: path.path.file_name()?
                .ok_or_else(|| anyhow!("No filename found in path"))?
                .into_boxed_str(),
            container: MemoryContainer::Loaded(
                PEContainer {
                    path: path.clone(),
                    image: Mutex::new(Arc::downgrade(&arc_img)),
                }
            ),
            dependencies,
        })
    }

    pub fn new_emulated(name: &str, exports: Vec<PEExportedFunction>) -> anyhow::Result<Self> {
        Ok(Self {
            name: name.into(),
            container: MemoryContainer::Emulated(
                EmuContainer {
                    emulated_exports: exports
                }
            ),
            dependencies: Vec::new()
        })
    }

    pub fn new_emulated_boxed(name: Box<str>, exports: Vec<PEExportedFunction>) -> anyhow::Result<Self> {
        Ok(Self {
            name,
            container: MemoryContainer::Emulated(
                EmuContainer {
                    emulated_exports: exports
                }
            ),
            dependencies: Vec::new()
        })
    }

    pub fn get_name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn get_export_addr_by_name(&self, name: &str) -> anyhow::Result<*const u8> {
        let exports = match &self.container {
            MemoryContainer::Loaded(pe) => {
                &pe.get_image_arc()?.exports
            }
            MemoryContainer::Emulated(emu) => {
                &emu.emulated_exports
            }
        };

        exports
            .iter()
            .find(|e| e.name.as_ref().map(|s| s.as_ref()) == Some(name))
            .map(|e| e.addr)
            .ok_or_else(|| anyhow!("Export {} from DLL {} not found", name, self.name))
    }

    pub fn get_export_addr_by_ordinal(&self, ordinal: u16) -> anyhow::Result<*const u8> {
        let exports = match &self.container {
            MemoryContainer::Loaded(pe) => {
                &pe.get_image_arc()?.exports
            }
            MemoryContainer::Emulated(emu) => {
                &emu.emulated_exports
            }
        };

        exports
            // Quick lookup
            .get(ordinal.saturating_sub(1) as usize)
            .filter(|e| e.ordinal == ordinal)
            // If the quick lookup fails, fallback to a linear search
            .or_else(|| exports.iter().find(|e| e.ordinal == ordinal))
            .map(|e| e.addr)
            .ok_or_else(|| anyhow!("Export ordinal {} from DLL {} not found", ordinal, self.name))
    }

    /// Clones the Arc.
    pub fn get_dependency(&self, own_ref: &ArcMemoryDLL) -> anyhow::Result<PEDependency> {
        Ok(PEDependency {
            dll: Arc::clone(own_ref),
            image: match &self.container {
                MemoryContainer::Loaded(pe) => Some(pe.get_image_arc()?),
                _ => None,
            },
        })
    }
}

impl PartialEq for InMemoryDLL {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PEContainer {
    pub fn get_image_arc(&self) -> anyhow::Result<Arc<PEMappedImage>> {
        let mut image = self.image.lock();

        if let Some(al_img) = image.upgrade() {
            return Ok(al_img)
        }

        let new_image = Arc::new(load_from_disk(&self.path)?.0);

        *image = Arc::downgrade(&new_image);

        Ok(new_image)
    }
}

impl PEMappedImage {
    pub fn new(loaded_image: LoadedImage, exports: Vec<PEExportedFunction>) -> Self {
        Self {
            loaded_image,
            exports,
        }
    }
}

impl Clone for PEDependency {
    fn clone(&self) -> Self {
        Self {
            dll: Arc::clone(&self.dll),
            image: match &self.image {
                Some(image) => Some(Arc::clone(image)),
                None => None
            },
        }
    }
}