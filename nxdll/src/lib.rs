#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

pub mod loader;
pub mod io;
pub mod exports;
pub mod utils;

use core::panic::PanicInfo;
use log::error;
use nxdk_rs::hal::debug::debug_print_str_ln;
use crate::utils::log::init_logger;

#[macro_use]
extern crate alloc;

#[global_allocator]
static ALLOCATOR: nxdk_rs::xbox_alloc::XboxKernelAlloc = nxdk_rs::xbox_alloc::XboxKernelAlloc {};

#[no_mangle]
pub extern "C" fn nx_loader_init() {
    // Ignore if the logger cannot initialize. Nowhere to log.
    let err = init_logger();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debug_print_str_ln(&format!("{}", info));
    error!("Panic: {}", info);

    loop {}
}