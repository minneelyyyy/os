
use crate::{boot, mem};

pub mod paging;

mod gdt;
mod idt;
mod irq;

pub struct ArchEntry {
    map: paging::PageMap,
    kernel_region: mem::MemoryRegion,
    alloc: boot::mem::EarlyBootAllocator,
}

pub unsafe fn arch_entry(entry: ArchEntry) -> ! {
    unsafe { gdt::init() };
    unsafe { idt::init() };

    unsafe { crate::kmain() };
}
