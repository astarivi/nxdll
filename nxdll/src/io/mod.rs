use crate::io::storage::mount::{get_storage, XboxStorage};
use lazy_static::lazy_static;

pub mod storage;
pub mod bufio;
pub mod threading;

lazy_static! {
    pub static ref INTERNAL_STORAGE: XboxStorage = get_storage();
}