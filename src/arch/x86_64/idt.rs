
use core::mem;
use core::arch::asm;

use bitfield_struct::bitfield;

use crate::printlnk;

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    offset: u64,
}

#[bitfield(u128)]
struct GateDescriptor {
    offset_lo: u16,
    #[bits(16)]
    segment_selector: super::SegmentSelector,
    #[bits(5)]
    _reserved: usize,
    #[bits(3)]
    ist: usize,
    #[bits(4)]
    gate_type: usize,
    _zero: bool,
    #[bits(2)]
    dpl: usize,
    present: bool,
    offset_mid: u16,
    offset_hi: u32,
    _reserved1: u32,
}

impl GateDescriptor {
    const fn with_offset(self, offset: u64) -> Self {
        self
            .with_offset_lo((offset & 0xffff) as u16)
            .with_offset_mid(((offset & 0xffff0000) >> 16) as u16)
            .with_offset_hi(((offset & 0xffffffff00000000) >> 32) as u32)
    }
}

static mut IDT: [GateDescriptor; 256] = [GateDescriptor::new(); 256];

#[repr(C, align(0x10))]
pub struct InterruptFrame {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,

    rsi: u64,
    rdi: u64,
    rbp: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,

    vector: u64,
    error_code: u64,

    rip: u64,
    cs: u64,
    rflags: u64,
}

unsafe extern "C" {
    fn isr6();
}

#[unsafe(no_mangle)]
extern "C" fn interrupt_handler(frame: *const InterruptFrame) {
    // In builds with the printlnk call from x86_64::init built in, this is a NULL pointer.
    // otherwise, it seems like a valid pointer, but accessing it causes it to crash.
    printlnk!("frame ptr = {:p}", frame);

    // causes either a panic or a crash.
    // printlnk!("vector = {}", unsafe { (*frame).vector })
}

pub fn init() {
    unsafe {
        IDT[6] = GateDescriptor::new()
            .with_offset(isr6 as *const () as u64)
            .with_segment_selector(super::SegmentSelector::new()
                .with_index(1)
                .with_rpl(0)
                .with_ti(false))
            .with_present(true)
            .with_gate_type(0xE);
    }

    let gdtr = Idtr {
        limit: (mem::size_of::<[GateDescriptor; 256]>() - 1) as u16,
        offset: &raw const IDT as u64,
    };

    unsafe {
        asm!(
            "lidt [{}]",
            in(reg) &raw const gdtr,
            options(readonly, nostack, preserves_flags)
        )
    };
}
