
use crate::boot;

pub struct KernelOwnPaging;

pub unsafe fn early_init(
    _alloc: &boot::mem::EarlyBootAllocator,
    _map: &boot::mem::MemoryMap) -> KernelOwnPaging
{
    todo!()
}

pub struct MappedHigherHalf;

pub unsafe fn map_higher_half_kernel(
    _cookie: KernelOwnPaging,
    _alloc: &boot::mem::EarlyBootAllocator,
    _kernel_region: &crate::mem::MemoryRegion) -> MappedHigherHalf
{
    todo!()
}
