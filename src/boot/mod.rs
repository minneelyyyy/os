
mod efi;
mod arch;
mod mem;

pub struct BootData {
    map: mem::MemoryMap,
    kernel_region: crate::mem::MemoryRegion,
}
