use core::arch::asm;

use bitfield_struct::bitfield;

use crate::printlnk;

mod gdt;
mod idt;

#[bitfield(u16)]
pub struct SegmentSelector {
    #[bits(2)]
    rpl: usize,
    ti: bool,
    #[bits(13)]
    index: usize,
}

pub unsafe fn hcf() -> ! {
    loop {
        unsafe { asm!("hlt") };
    }
}

pub unsafe fn outb(port: u16, byte: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") byte,
            options(nomem, nostack, preserves_flags)
        )
    };
}

pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;

    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        )
    };

    value
}

pub fn init() {
    gdt::init();
    idt::init();

    // This line seems to affect the behaviour caused by the following
    // ud2 call for some reason.
    // printlnk!("IDT Initialized.");

    unsafe {
        asm!("ud2")
    };
}
