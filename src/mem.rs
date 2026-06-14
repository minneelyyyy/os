
pub const PAGE_OFFSET: usize = 12;
pub const PAGE_MASK: usize = (1 << PAGE_OFFSET) - 1;
pub const PAGE_SIZE: usize = 4096;

#[repr(C, align(4096))]
pub struct Page(pub [u8; 4096]);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub base: *mut Page,
    pub npages: usize,
}

impl MemoryRegion {
    pub fn new(base: *mut Page, npages: usize) -> Self {
        Self { base, npages }
    }

    /// Move the base page of a memory region to a new location, keeping size information.
    pub fn rebased(mut self, base: *mut Page) -> Self {
        self.base = base;
        self
    }

    pub fn iter(&self) -> PageIter {
        PageIter { base: self.base, npages: self.npages }
    }

    /// Returns whether two regions overlap in any way or not.
    pub fn overlaps(self, other: Self) -> bool {
        todo!()
    }
}

pub struct PageIter {
    base: *mut Page,
    npages: usize,
}

impl Iterator for PageIter {
    type Item = *mut Page;

    fn next(&mut self) -> Option<Self::Item> {
        (self.npages > 0).then(move || {
            let p = self.base;
            self.base = unsafe { self.base.add(1) };
            self.npages -= 1;
            
            p
        })
    }
}

pub type VirtAddr = usize;
pub type PhysAddr = usize;
