
mod efi;
mod arch;
mod mem;
mod entry;

pub struct BootData {
    map: mem::MemoryMap,
    kernel_region: crate::mem::MemoryRegion,
}
