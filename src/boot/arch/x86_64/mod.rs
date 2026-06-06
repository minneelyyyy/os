
use crate::boot;

pub mod paging;

mod gdt;
mod idt;
mod irq;

pub unsafe fn perform_higher_half_jump(
    _cookie: paging::MappedHigherHalf,
    _info: boot::BootData,
    _early: boot::mem::EarlyBootAllocator) -> !
{
    todo!()
}

pub unsafe fn arch_entry(_info: boot::BootData) -> ! {
    todo!()
}
