
use crate::{boot, printlnk};
use boot::arch;

/// Proc entered by the UEFI stub.
/// We must first remap the kernel as higher half and jump to it.
pub unsafe fn uefi_entry(info: boot::BootData) -> ! {
    printlnk!("KERNEL MAP:");
    for region in unsafe { info.map.as_slice() } {
        printlnk!("Base: {:016p} Number of Pages: {:10}  {:?}",
            region.region.base, region.region.npages, region.region_type);
    }

    printlnk!("KERNEL IMAGE: Base {:p} Number of Pages {}",
        info.kernel_region.base, info.kernel_region.npages);

    let early = boot::mem::EarlyBootAllocator::init(&info.map);

    let m = unsafe { arch::paging::early_init(&early, &info.map) };
    let hh = unsafe { arch::paging::map_higher_half_kernel(m, &early, &info.kernel_region) };

    unsafe { arch::perform_higher_half_jump(hh, info, early) };
}
