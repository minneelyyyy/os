
use crate::boot;
use core::arch::asm;

use bitfield_struct::bitfield;

#[bitfield(u64)]
struct PageMapLevel4Entry {
    present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    _ignored1: bool,
    _reserved: bool,
    #[bits(4)]
    _ignored2: usize,
    #[bits(40)]
    addr: usize,
    #[bits(11)]
    _ignored3: usize,
    execute_disabled: bool,
}

#[bitfield(u64)]
struct PageDirectoryPointerTableEntry {
    present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    dirty: bool,
    gb_page: bool,
    global: bool,
    #[bits(2)]
    _ignored1: usize,
    pat: bool,
    #[bits(40)]
    addr: usize,
    #[bits(7)]
    _ignored2: usize,
    #[bits(4)]
    pk: usize,
    execute_disabled: bool,
}

#[bitfield(u64)]
struct PageDirectoryEntry {
    present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    dirty: bool,
    mb2_page: bool,
    global: bool,
    #[bits(2)]
    _ignored1: usize,
    _r: bool,
    pat: bool,
    #[bits(8)]
    _reserved: usize,
    #[bits(31)]
    addr: usize,
    #[bits(7)]
    _ignored2: usize,
    #[bits(4)]
    pk: usize,
    execute_disabled: bool,
}

#[bitfield(u64)]
struct PageTableEntry {
    present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    dirty: bool,
    pat: bool,
    global: bool,
    #[bits(2)]
    _ignored1: usize,
    _r: bool,
    #[bits(40)]
    addr: usize,
    #[bits(7)]
    _ignored2: usize,
    #[bits(4)]
    pk: usize,
    execute_disabled: bool,
}

unsafe fn la57_enabled() -> bool {
    let mut cr4: usize = 0;

    unsafe {
        asm!(
            "mov {}, cr4",
            out(reg) cr4,
        );
    }

    cr4 & (1 << 12) != 0
}

pub struct EarlyPagingInit;

pub unsafe fn early_init(
    _alloc: &boot::mem::EarlyBootAllocator,
    _map: &boot::mem::MemoryMap) -> EarlyPagingInit
{
    // kernel does not currently support 5-level paging
    assert!(unsafe { !la57_enabled() });

    todo!()
}

pub struct MappedHigherHalf;

pub unsafe fn map_higher_half_kernel(
    _cookie: EarlyPagingInit,
    _alloc: &boot::mem::EarlyBootAllocator,
    _kernel_region: &crate::mem::MemoryRegion) -> MappedHigherHalf
{
    todo!()
}
