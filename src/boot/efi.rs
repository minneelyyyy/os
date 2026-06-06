use crate::{mem::MemoryRegion, printlnk};
use crate::boot::mem;

use core::{ptr, time};

use r_efi::{base, efi::{self, BootServices}};

const PAGE_SIZE: usize = 4096;

unsafe fn get_image_region(handle: efi::Handle, bs: &BootServices) -> MemoryRegion {
    let mut guid = efi::protocols::loaded_image::PROTOCOL_GUID;
    let mut img: *mut r_efi::protocols::loaded_image::Protocol = ptr::null_mut();

    unsafe {
        let r = (bs.open_protocol)(
            handle,
            &raw mut guid,
            &raw mut img as *mut _,
            handle,
            ptr::null_mut(),
            efi::OPEN_PROTOCOL_EXCLUSIVE
        );

        if r != efi::Status::SUCCESS {
            panic!("Failure to call OpenProtocol for LoadedImage ({:?} {})", r, r.as_usize());
        }
    }

    let loaded_image = unsafe { &*img };

    MemoryRegion::new(
        loaded_image.image_base as *mut _,
        (loaded_image.image_size as usize).div_ceil(PAGE_SIZE))
}

unsafe fn allocate_pages_any_loader_data<T>(bs: &BootServices, pages: usize) -> *mut T {
    let mut ptr = 0;
    unsafe { (bs.allocate_pages)(efi::ALLOCATE_ANY_PAGES, efi::LOADER_DATA, pages, &mut ptr) };
    ptr as *mut T
}

unsafe fn get_efi_memory_map_size(bs: &BootServices) -> (usize, usize) {
    let mut size = 0;
    let mut map_key = 0;
    let mut desc_size = 0;
    let mut desc_version = 0;

    unsafe {
        let r = (bs.get_memory_map)(&raw mut size, ptr::null_mut(), &raw mut map_key, &raw mut desc_size, &raw mut desc_version);
        if r != efi::Status::BUFFER_TOO_SMALL {
            panic!("failed to get memory map size");
        }
    };

    assert_eq!(size % desc_size, 0);

    (size / desc_size, desc_size)
}

unsafe fn get_efi_memory_map(bs: &BootServices, buf: *mut efi::MemoryDescriptor, nbytes: usize) -> Option<(usize, usize)> {
    let mut size = nbytes;
    let mut map_key = 0;
    let mut desc_size = 0;
    let mut desc_version = 0;

    unsafe {
        let r = (bs.get_memory_map)(&raw mut size, buf, &raw mut map_key, &raw mut desc_size, &raw mut desc_version);

        match r {
            efi::Status::SUCCESS => { return Some((size / desc_size, map_key)) },
            efi::Status::BUFFER_TOO_SMALL => return None,
            _ => panic!("unexpected error getting memory map"),
        }
    };
}

unsafe fn create_memory_map_for_kernel(map: *const efi::MemoryDescriptor, buf: *mut mem::BootMappedRegion, n: usize, elem_size: usize) {
    for i in 0..n {
        let region = unsafe { map.byte_add(i * elem_size) };

        let rtype = match unsafe { (*region).r#type } {
            efi::BOOT_SERVICES_CODE | efi::BOOT_SERVICES_DATA |
            efi::LOADER_CODE | efi::LOADER_DATA => mem::BootMemoryRegionType::Reclaimable,
            efi::ACPI_RECLAIM_MEMORY => mem::BootMemoryRegionType::ACPIReclaimable,
            efi::CONVENTIONAL_MEMORY => mem::BootMemoryRegionType::Free,
            efi::MEMORY_MAPPED_IO => mem::BootMemoryRegionType::MemoryMappedIO,
            _ => mem::BootMemoryRegionType::Reserved,
        };

        let out = unsafe { buf.add(i) };
        unsafe {
            *out = mem::BootMappedRegion::new(
                rtype,
                MemoryRegion::new(
                    (*region).physical_start as *mut _,
                    (*region).number_of_pages as usize))
        };
    }
}

#[unsafe(no_mangle)]
unsafe extern "efiapi" fn efi_main(h: efi::Handle, st: *mut efi::SystemTable) -> efi::Status {
    let bs = unsafe { (*st).boot_services };
    let bs = unsafe { bs.as_ref().unwrap() };

    let image_region = unsafe { get_image_region(h, bs) };

    for _ in 0..2 {
        let (mut nentries, entrysz) = unsafe { get_efi_memory_map_size(bs) };
        nentries += 8;

        let nbytes_memory_map = nentries * entrysz;
        let npages_memory_map = (nbytes_memory_map + PAGE_SIZE - 1) / PAGE_SIZE;
        let memory_map = unsafe { allocate_pages_any_loader_data(bs, npages_memory_map) };

        let nbytes_kernel_map = nentries * core::mem::size_of::<mem::BootMappedRegion>();
        let npages_kernel_map = (nbytes_kernel_map + PAGE_SIZE - 1) / PAGE_SIZE;
        let kernel_map = unsafe { allocate_pages_any_loader_data(bs, npages_kernel_map) };

        let (size, key) = unsafe { get_efi_memory_map(bs, memory_map, nbytes_memory_map).unwrap() };
        unsafe { create_memory_map_for_kernel(memory_map, kernel_map, size, entrysz) };

        match unsafe { (bs.exit_boot_services)(h, key) } {
            base::Status::SUCCESS => {
                unsafe {
                    super::entry::uefi_entry(super::BootData {
                        map: mem::MemoryMap::new(kernel_map, size),
                        kernel_region: image_region,
                    });
                }
            },
            base::Status::INVALID_PARAMETER => {},
            _ => panic!("invalid status returned by ExitBootServices"),
        }

        unsafe {
            (bs.free_pages)(memory_map as u64, npages_memory_map);
            (bs.free_pages)(kernel_map as u64, npages_kernel_map);
        }
    }

    printlnk!("failed call to ExitBootServices(). Exiting in 10 seconds.");
    unsafe { (bs.stall)(time::Duration::from_secs(10).as_micros() as usize) };

    efi::Status::SUCCESS
}
