#![no_main]
#![no_std]

use core::panic::PanicInfo;

mod efi;
mod memory;
mod serial;

#[cfg(target_arch = "x86_64")]
pub mod arch {
    mod x86_64;
    pub use x86_64::*;
}

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => {
        let mut writer = crate::serial::SerialWriter;
        let _ = core::fmt::write(&mut writer, core::format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! printlnk {
    () => {
        $crate::printk!("\n");
    };
    ($($arg:tt)*) => {
        $crate::printk!($($arg)*);
        $crate::printk!("\n");
    }
}

pub struct BootData {
    map: memory::MemoryMap,
}

pub fn kmain(_data: BootData) -> ! {
    arch::init();
    printlnk!("CPU Initialized.");

    unsafe { arch::hcf() };
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    printlnk!("{}", info);
    unsafe { arch::hcf() };
}
