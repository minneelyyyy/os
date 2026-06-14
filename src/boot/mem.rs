
use crate::mem::{MemoryRegion, Page};
use core::slice;

#[derive(Debug)]
pub enum BootMemoryRegionType {
    Reclaimable,
    ACPIReclaimable,
    Free,
    MemoryMappedIO,
    Reserved,
}

#[derive(Debug)]
pub struct BootMappedRegion {
    pub region_type: BootMemoryRegionType,
    pub region: MemoryRegion,
}

impl BootMappedRegion {
    pub fn new(region_type: BootMemoryRegionType, region: MemoryRegion) -> Self {
        Self { region_type, region }
    }
}

pub struct MemoryMap {
    len: usize,
    ptr: *const BootMappedRegion,
}

impl MemoryMap {
    pub unsafe fn new(ptr: *const BootMappedRegion, len: usize) -> Self {
        Self { len, ptr }
    }

    pub unsafe fn as_slice(&self) -> &[BootMappedRegion] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}


use core::ptr;

#[repr(C)]
pub struct EarlyBootAllocator {
    head: *mut BumpRegionMeta,
}

#[repr(align(4096))]
struct BumpRegionMeta {
    next: *mut BumpRegionMeta,
    base: *mut Page,
    top: *mut Page,
    npages: usize,
}

impl EarlyBootAllocator {
    pub fn init(map: &MemoryMap) -> Self {
        let free_regions = unsafe { map.as_slice() }.iter()
            .filter(|r| matches!(r.region_type, BootMemoryRegionType::Free));

        let (head, _) = free_regions.fold(
            (ptr::null_mut(), ptr::null_mut()),
            |(head, tail): (*mut BumpRegionMeta, *mut BumpRegionMeta), region: &BootMappedRegion| {
                let region = &region.region;

                if region.base.is_null() || region.npages < 2 {
                    return (head, tail);
                }

                // SAFETY: BumpRegionMeta is a page aligned type, as is region.base
                let meta: *mut BumpRegionMeta = region.base as *mut _;

                unsafe {
                    (*meta).next = ptr::null_mut();
                    (*meta).base = region.base.add(1);  // page after meta is the start of free region
                    (*meta).top = (*meta).base;
                    (*meta).npages = region.npages - 1; // npages does not include meta page
                };

                if head.is_null() {
                    return (meta, ptr::null_mut());
                }

                let parent = if tail.is_null() { head } else { tail };

                unsafe { (*parent).next = meta };
                (head, meta)
            }
        );

        Self { head }
    }

    unsafe fn get_usable_region_meta(&mut self) -> *mut BumpRegionMeta {
        let mut region = self.head;

        while !region.is_null() {
            let first_inval_page = unsafe { (*region).base.add((*region).npages) };

            if unsafe { (*region).top } < first_inval_page {
                break;
            } else {
                region = unsafe { (*region).next };
            }
        }

        region
    }

    pub unsafe fn get_page(&mut self) -> *mut Page {
        let usable_region = unsafe { self.get_usable_region_meta() };
        if usable_region.is_null() {
            return ptr::null_mut();
        }

        let page = unsafe { (*usable_region).top } as *mut _;
        unsafe { (*usable_region).top = (*usable_region).top.add(1) };

        page
    }

    pub unsafe fn get_page_zeroed(&mut self) -> *mut Page {
        let page = unsafe { self.get_page() };
        unsafe { (*page).0.fill(0x0) };
        page
    }
}
