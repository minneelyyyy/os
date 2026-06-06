
#[repr(align(4096))]
pub struct Page([u8; 4096]);

#[derive(Debug)]
pub struct MemoryRegion {
    pub base: *mut Page,
    pub npages: usize,
}

impl MemoryRegion {
    pub fn new(base: *mut Page, npages: usize) -> Self {
        Self { base, npages }
    }
}
