#![no_main]
#![no_std]

use core::panic::PanicInfo;

mod mem;
mod serial;
mod arch;
mod boot;

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

pub unsafe fn kmain() -> ! {
    printlnk!("Hello, world!");
    unsafe { arch::hcf() };
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    printlnk!("{}", info);
    unsafe { arch::hcf() };
}
