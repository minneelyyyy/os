
use crate::printlnk;
use crate::boot;

mod gdt;
mod idt;
mod irq;
mod page;

unsafe fn perform_higher_half_jump(info: boot::BootData, early: boot::mem::EarlyBootAllocator) -> ! {
    todo!()
}

pub unsafe fn entry(info: boot::BootData) -> ! {
    printlnk!("KERNEL MAP:");
    for region in unsafe { info.map.as_slice() } {
        printlnk!("Base: 0x{:016p} Number of Pages: {:10}  {:?}",
            region.region.base, region.region.npages, region.region_type);
    }

    printlnk!("KERNEL IMAGE: Base 0x{:016p} Number of Pages {}",
        info.kernel_region.base, info.kernel_region.npages);

    let early = boot::mem::EarlyBootAllocator::init(&info.map);

    unsafe { page::init_early_boot_pages(&early, &info.map, &info.kernel_region) };
    unsafe { perform_higher_half_jump(info, early) };
}
