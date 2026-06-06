use core::arch::asm;

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
