use crate::{memory::{self, MemoryMap}, printlnk};

use core::{ptr, time};

use r_efi::{base, efi::{self, BootServices}};

const PAGE_SIZE: usize = 4096;

fn allocate_pages_any_loader_data<T>(bs: &BootServices, pages: usize) -> *mut T {
    let mut ptr = 0;
    unsafe { (bs.allocate_pages)(efi::ALLOCATE_ANY_PAGES, efi::LOADER_DATA, pages, &mut ptr) };
    ptr as *mut T
}

fn get_efi_memory_map_size(bs: &BootServices) -> (usize, usize) {
    let mut map_size = 0;
    let mut map_key = 0;
    let mut desc_size = 0;
    let mut desc_version = 0;

    unsafe {
        let r = (bs.get_memory_map)(&raw mut map_size, ptr::null_mut(), &raw mut map_key, &raw mut desc_size, &raw mut desc_version);
        if r != efi::Status::BUFFER_TOO_SMALL {
            panic!("failed to get memory map size");
        }
    };

    assert_eq!(map_size % desc_size, 0);

    (map_size / desc_size, desc_size)
}

fn get_efi_memory_map(bs: &BootServices, buf: *mut efi::MemoryDescriptor, nbytes: usize) -> Option<(usize, usize)> {
    let mut size = nbytes;
    let mut map_key = 0;
    let mut desc_size = 0;
    let mut desc_version = 0;

    unsafe {
        let r = (bs.get_memory_map)(&raw mut size, buf, &raw mut map_key, &raw mut desc_size, &raw mut desc_version);

        match r {
            efi::Status::SUCCESS => {return Some((size / desc_size, map_key))},
            efi::Status::BUFFER_TOO_SMALL => return None,
            _ => panic!("unexpected error getting memory map"),
        }
    };
}

fn create_memory_map_for_kernel(map: *const efi::MemoryDescriptor, buf: *mut memory::MemoryRegion, n: usize) {
    for i in 0..n {
        let region = unsafe { map.add(i) };

        let rtype = match unsafe { (*region).r#type } {
            efi::LOADER_CODE | efi::LOADER_DATA => memory::MemoryRegionType::Reclaimable,
            efi::ACPI_RECLAIM_MEMORY => memory::MemoryRegionType::ACPIReclaimable,
            efi::CONVENTIONAL_MEMORY => memory::MemoryRegionType::Free,
            efi::MEMORY_MAPPED_IO => memory::MemoryRegionType::MemoryMappedIO,
            _ => memory::MemoryRegionType::Reserved,
        };

        let out = unsafe { buf.add(i) };
        unsafe {
            *out = memory::MemoryRegion::new(rtype, (*region).physical_start as usize, (*region).number_of_pages as usize)
        };
    }
}

#[unsafe(no_mangle)]
unsafe extern "efiapi" fn efi_main(h: efi::Handle, st: *mut efi::SystemTable) -> efi::Status {
    let bs = unsafe { (*st).boot_services };
    let bs = unsafe { bs.as_ref().unwrap() };

    for _ in 0..2 {
        let (mut nentries, entrysz) = get_efi_memory_map_size(bs);
        nentries += 8;

        let nbytes_memory_map = nentries * entrysz;
        let npages_memory_map = (nbytes_memory_map + PAGE_SIZE - 1) / PAGE_SIZE;
        let memory_map = allocate_pages_any_loader_data(bs, npages_memory_map);

        let nbytes_kernel_map = nentries * core::mem::size_of::<memory::MemoryRegion>();
        let npages_kernel_map = (nbytes_kernel_map + PAGE_SIZE - 1) / PAGE_SIZE;
        let kernel_map = allocate_pages_any_loader_data(bs, npages_kernel_map);

        let (size, key) = get_efi_memory_map(bs, memory_map, nbytes_memory_map).unwrap();
        create_memory_map_for_kernel(memory_map, kernel_map, size);

        match unsafe { (bs.exit_boot_services)(h, key) } {
            base::Status::SUCCESS => {
                crate::kmain(crate::BootData {
                    map: unsafe { MemoryMap::new(kernel_map, size) }
                });
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
