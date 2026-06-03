
use core::mem;
use core::arch::asm;

use bitfield_struct::bitfield;

#[bitfield(u64)]
struct SegmentDescriptor {
    limit_lo: u16,
    base_lo: u16,
    base_hi_lo: u8,
    access_byte: u8,
    #[bits(4)]
    limit_hi: usize,
    #[bits(4)]
    flags: usize,
    base_hi_hi: u8,
}

impl SegmentDescriptor {
    const fn with_base(self, base: u32) -> Self {
        self
            .with_base_lo((base & 0xffff) as u16)
            .with_base_hi_lo(((base & 0xff0000) >> 16) as u8)
            .with_base_hi_hi(((base & 0xff000000) >> 24) as u8)
    }

    const fn with_limit(self, limit: u32) -> Self {
        self
            .with_limit_lo((limit & 0xffff) as u16)
            .with_limit_hi(((limit & 0xf0000) >> 16) as usize)
    }
}

#[repr(C, packed)]
struct TssDescriptor {
    high: SegmentDescriptor,
    base_hi: u32,
    _reserved: u32,
}

impl TssDescriptor {
    const fn new() -> Self {
        Self { high: SegmentDescriptor::new(), base_hi: 0, _reserved: 0 }
    }

    const fn with_base(mut self, base: u64) -> Self {
        self.high = self.high
            .with_base_lo((base & 0xffff) as u16)
            .with_base_hi_lo(((base & 0xff0000) >> 16) as u8)
            .with_base_hi_hi(((base & 0xff000000) >> 24) as u8);

        self.base_hi = ((base & 0xffffffff00000000) >> 32) as u32;
        self
    }

    const fn with_limit(mut self, limit: u32) -> Self {
        self.high = self.high
            .with_limit_lo((limit & 0xffff) as u16)
            .with_limit_hi(((limit & 0xf0000) >> 16) as usize);

        self
    }

    const fn with_access_byte(mut self, access_byte: u8) -> Self {
        self.high = self.high.with_access_byte(access_byte);
        self
    }

    const fn with_flags(mut self, flags: usize) -> Self {
        self.high = self.high.with_flags(flags);
        self
    }
}

#[repr(C, packed)]
struct Gdt {
    null: SegmentDescriptor,
    kernel_code: SegmentDescriptor,
    kernel_data: SegmentDescriptor,
    user_code: SegmentDescriptor,
    user_data: SegmentDescriptor,
    tss: TssDescriptor,
}

static mut GDT: Gdt = Gdt {
    null: SegmentDescriptor::new()
        .with_base(0x0)
        .with_limit(0xfffff)
        .with_access_byte(0x0)
        .with_flags(0x0),
    kernel_code: SegmentDescriptor::new()
        .with_base(0x0)
        .with_limit(0xfffff)
        .with_access_byte(0x9a)
        .with_flags(0xa),
    kernel_data: SegmentDescriptor::new()
        .with_base(0x0)
        .with_limit(0xfffff)
        .with_access_byte(0x92)
        .with_flags(0xc),
    user_code: SegmentDescriptor::new()
        .with_base(0x0)
        .with_limit(0xfffff)
        .with_access_byte(0xf2)
        .with_flags(0xc),
    user_data: SegmentDescriptor::new()
        .with_base(0x0)
        .with_limit(0xfffff)
        .with_access_byte(0xfa)
        .with_flags(0xa),
    tss: TssDescriptor::new(),
};

#[repr(C, packed)]
pub struct Tss {
    reserved0: u32,

    rsp0: u64,
    rsp1: u64,
    rsp2: u64,

    reserved1: u64,

    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,

    reserved2: u64,
    reserved3: u16,

    io_map_base: u16,
}

static mut TSS: Tss = Tss {
    reserved0: 0,

    rsp0: 0,
    rsp1: 0,
    rsp2: 0,

    reserved1: 0,

    ist1: 0,
    ist2: 0,
    ist3: 0,
    ist4: 0,
    ist5: 0,
    ist6: 0,
    ist7: 0,

    reserved2: 0,
    reserved3: 0,

    io_map_base: core::mem::size_of::<Tss>() as u16,
};

#[repr(C, packed)]
struct GdtDescriptor {
    limit: u16,
    offset: u64,
}

pub fn init() {
    unsafe { 
        GDT.tss = TssDescriptor::new()
            .with_base(&raw const TSS as u64)
            .with_limit(mem::size_of::<Tss>() as u32 - 1)
            .with_access_byte(0x89)
            .with_flags(0x0);
    };

    let gdtr = GdtDescriptor {
        limit: (mem::size_of::<Gdt>() - 1) as u16,
        offset: &raw const GDT as u64,
    };

    unsafe {
        asm!(
            "lgdt [{}]",
            in(reg) &raw const gdtr,
            options(readonly, nostack, preserves_flags)
        )
    };

    unsafe {
        asm!(
            "push 0x08",
            "lea rax, [2f]",
            "push rax",
            "retfq",
            "2:",
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            out("rax") _,
        )
    };
}
