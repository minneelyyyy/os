use core::slice;


pub enum MemoryRegionType {
    Reclaimable,
    ACPIReclaimable,
    Free,
    MemoryMappedIO,
    Reserved,
}

pub struct MemoryRegion {
    pub region_type: MemoryRegionType,
    pub base: usize,
    pub len: usize,
}

impl MemoryRegion {
    pub fn new(region_type: MemoryRegionType, base: usize, len: usize) -> Self {
        Self { region_type, base, len }
    }
}

pub struct MemoryMap {
    len: usize,
    ptr: *const MemoryRegion,
}

impl MemoryMap {
    pub unsafe fn new(ptr: *const MemoryRegion, len: usize) -> Self {
        Self { len, ptr }
    }

    pub unsafe fn as_slice(&self) -> &[MemoryRegion] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}
