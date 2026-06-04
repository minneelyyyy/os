
use core::mem;
use core::arch::asm;

use bitfield_struct::bitfield;

use super::irq;

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

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    offset: u64,
}

pub fn init() {
    unsafe {
        IDT[6] = GateDescriptor::new()
            .with_offset(irq::invalid_opcode as *const () as u64)
            .with_segment_selector(super::SegmentSelector::new()
                .with_index(1)
                .with_rpl(0)
                .with_ti(false))
            .with_present(true)
            .with_gate_type(0xE);

        IDT[14] = GateDescriptor::new()
            .with_offset(irq::page_fault as *const () as u64)
            .with_segment_selector(super::SegmentSelector::new()
                .with_index(1)
                .with_rpl(0)
                .with_ti(false))
            .with_present(true)
            .with_gate_type(0xE);
    }

    let idtr = Idtr {
        limit: (mem::size_of::<[GateDescriptor; 256]>() - 1) as u16,
        offset: &raw const IDT as u64,
    };

    unsafe {
        asm!(
            "lidt [{}]",
            in(reg) &raw const idtr,
            options(readonly, nostack, preserves_flags)
        )
    };
}
