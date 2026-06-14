
use crate::{
    boot::{self, arch::ArchEntry},
    mem::{
        MemoryRegion, PAGE_MASK, PAGE_OFFSET, Page, PhysAddr, VirtAddr
    },
    printlnk,
};

use core::{
    arch::{asm, naked_asm}, error, fmt, ptr::NonNull
};

use bitfield_struct::bitfield;

const MAX_VIRT_ADDR_NBITS: usize = 48;
#[allow(unused)]
const MAX_VIRT_ADDR: usize = (1 << MAX_VIRT_ADDR_NBITS) - 1;

#[derive(Debug, Clone)]
pub enum Error {
    AddrAlreadyMapped(VirtAddr),
    TableReferencesNull,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::AddrAlreadyMapped(addr) => write!(f, "{addr} is already mapped."),
            Self::TableReferencesNull => write!(f, "table entry references null ptr."),
        }
    }
}

impl error::Error for Error {}

#[bitfield(u64)]
struct LinearAddressPageTranslation {
    #[bits(12)]
    offset: usize,
    #[bits(9)]
    table_idx: usize,
    #[bits(9)]
    dir_idx: usize,
    #[bits(9)]
    dir_ptr_idx: usize,
    #[bits(9)]
    pml4_idx: usize,
    #[bits(16)]
    _unused: usize,
}

#[derive(Debug, Clone, Copy)]
enum Mapping {
    Page4K(PageTableEntry),
    Page2M(Huge2MPageRef),
    Page1G(Huge1GPageRef),
}

impl Mapping {
    fn npages(&self) -> usize {
        match self {
            Self::Page4K(_) => 1,
            Self::Page2M(_) => 512,
            Self::Page1G(_) => 512 * 512,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Perms {
    write: bool,
    exec: bool,
}

impl Perms {
    const WX: Perms = Perms { write: true, exec: true };
    const WO: Perms = Perms { write: true, exec: false };
}

#[repr(C)]
pub struct PageMap {
    base: NonNull<[PageMapLevel4EntryBare; 512]>,
}

impl PageMap {
    fn new(alloc: &mut boot::mem::EarlyBootAllocator) -> Self {
        let base = NonNull::new(
            unsafe { alloc.get_page_zeroed() as *mut _ }
        ).expect("Out of memory");

        Self { base }
    }

    /// Get a address mapping from a linear address by walking the page table.
    /// Returns None if the addr is unmapped.
    #[allow(unused)]
    unsafe fn virt_to_mapping(&self, addr: VirtAddr) -> Option<(Mapping, PhysAddr)> {
        let trans = LinearAddressPageTranslation::from_bits(addr as u64);

        let pml4 = unsafe { self.base.as_ref() };
        let pml4e = PageMapLevel4Entry::from_bare(pml4[trans.pml4_idx()])?;

        let dir_ptr_table = unsafe { pml4e.page_dir_ptr_table()?.as_ref() };
        let dir_ptr_entry = PageDirectoryPointerEntry::from_bare(dir_ptr_table[trans.dir_ptr_idx()])?;

        let dir_table = match dir_ptr_entry {
            PageDirectoryPointerEntry::NextLevel(table) => unsafe { table.page_dir_table()?.as_ref() },
            PageDirectoryPointerEntry::Huge(huge) => return Some((
                Mapping::Page1G(huge),
                huge.addr() | (addr & PAGE_1G_MASK),
            )),
        };

        let dir_entry = PageDirectoryEntry::from_bare(dir_table[trans.dir_idx()])?;

        let table = match dir_entry {
            PageDirectoryEntry::NextLevel(table) => unsafe { table.page_table()?.as_ref() },
            PageDirectoryEntry::Huge(huge) => return Some((
                Mapping::Page2M(huge),
                huge.addr() | (addr & PAGE_2M_MASK),
            )),
        };

        let table_entry = PageTableEntry::from_bare(table[trans.table_idx()])?;
        Some((
            Mapping::Page4K(table_entry),
            table_entry.addr() | (addr & PAGE_MASK),
        ))
    }

    unsafe fn map_page(
        &mut self,
        alloc: &mut boot::mem::EarlyBootAllocator,
        addr: VirtAddr,
        page: *const Page,
        perms: Perms) -> Result<(), Error>
    {
        let trans = LinearAddressPageTranslation::from_bits(addr as u64);

        let pml4 = unsafe { self.base.as_mut() };
        let pml4e = PageMapLevel4Entry::from_bare_or_overwrite(
            &mut pml4[trans.pml4_idx()],
            PageMapLevel4Entry::new()
                .with_addr(unsafe { alloc.get_page_zeroed() as VirtAddr })
                .with_write_enabled(true)
        );

        let dir_ptr_table = unsafe { pml4e.page_dir_ptr_table().ok_or(Error::TableReferencesNull)?.as_mut() };
        let dir_ptr_entry = PageDirectoryPointerEntry::from_bare_or_overwrite(
            &mut dir_ptr_table[trans.dir_ptr_idx()],
            PageDirectoryPointerEntry::NextLevel(
                PageDirRef::new()
                    .with_addr(unsafe { alloc.get_page_zeroed() as VirtAddr })
                    .with_write_enabled(true)
            )
        );

        let dir_table = match dir_ptr_entry {
            PageDirectoryPointerEntry::NextLevel(page_dir_ref) => unsafe {
                page_dir_ref.page_dir_table().ok_or(Error::TableReferencesNull)?.as_mut()
            },
            PageDirectoryPointerEntry::Huge(_) => return Err(Error::AddrAlreadyMapped(addr)),
        };

        let dir = PageDirectoryEntry::from_bare_or_overwrite(
            &mut dir_table[trans.dir_idx()],
            PageDirectoryEntry::NextLevel(
                PageTableRef::new()
                    .with_addr(unsafe { alloc.get_page_zeroed() as VirtAddr })
                    .with_write_enabled(true)
            )
        );

        let table = match dir {
            PageDirectoryEntry::NextLevel(page_table_ref) => unsafe {
                page_table_ref.page_table().ok_or(Error::TableReferencesNull)?.as_mut()
            },
            PageDirectoryEntry::Huge(_) => return Err(Error::AddrAlreadyMapped(addr)),
        };

        let entry = &mut table[trans.table_idx()];
        if entry.present() {
            return Err(Error::AddrAlreadyMapped(addr));
        }

        *entry = PageTableEntry::new()
            .with_addr(page as PhysAddr)
            .with_write_enabled(perms.write)
            .with_execute_disabled(!perms.exec)
            .to_bare();

        Ok(())
    }

    /// Very slow duplicate of an entire page table.
    unsafe fn duplicate(&self, alloc: &mut boot::mem::EarlyBootAllocator) -> PageMap {
        let dest_pml4 = unsafe {
            (alloc.get_page_zeroed() as *mut [PageMapLevel4EntryBare; 512]).as_mut().unwrap()
        };
        let src_pml4 = unsafe { self.base.as_ref() };

        // Page Map Level 4
        for i in 0..512 {
            let Some(src_pml4e) = PageMapLevel4Entry::from_bare(src_pml4[i]) else {
                continue;
            };

            let dest_pdpt = unsafe {
                (alloc.get_page_zeroed() as *mut [PageDirectoryPointerEntryBare; 512]).as_mut().unwrap()
            };

            let src_pdpt = unsafe { src_pml4e.page_dir_ptr_table().unwrap().as_ref() };

            // Page Directory Pointer Table
            for j in 0..512 {
                let Some(src_pdpte) = PageDirectoryPointerEntry::from_bare(src_pdpt[j]) else {
                    continue;
                };

                if let PageDirectoryPointerEntry::Huge(_) = src_pdpte {
                    dest_pdpt[j] = src_pdpte.to_bare();
                    continue;
                }

                let dest_pdt = unsafe {
                    (alloc.get_page_zeroed() as *mut [PageDirectoryEntryBare; 512]).as_mut().unwrap()
                };

                let PageDirectoryPointerEntry::NextLevel(src_pdtr) = src_pdpte else {
                    unreachable!()
                };

                let src_pdt = unsafe { src_pdtr.page_dir_table().unwrap().as_ref() };

                // Page Directory Table
                for k in 0..512 {
                    let Some(src_pdte) = PageDirectoryEntry::from_bare(src_pdt[k]) else {
                        continue;
                    };

                    if let PageDirectoryEntry::Huge(_) = src_pdte {
                        dest_pdt[k] = src_pdte.to_bare();
                        continue;
                    }

                    let dest_pt = unsafe {
                        (alloc.get_page_zeroed() as *mut [PageTableEntryBare; 512]).as_mut().unwrap()
                    };

                    let PageDirectoryEntry::NextLevel(src_ptr) = src_pdte else {
                        unreachable!()
                    };

                    let src_pt = unsafe { src_ptr.page_table().unwrap().as_ref() };

                    // Page Table
                    for l in 0..512 {
                        let Some(src_pte) = PageTableEntry::from_bare(src_pt[l]) else {
                            continue;
                        };

                        dest_pt[l] = src_pte.to_bare();
                    }

                    dest_pdt[k] = PageDirectoryEntry::NextLevel(
                        PageTableRef::from_bits(src_ptr.into_bits())
                            .with_addr(dest_pt.as_ptr() as usize)
                    ).to_bare();
                } 

                dest_pdpt[j] = PageDirectoryPointerEntry::NextLevel(
                    PageDirRef::from_bits(src_pdtr.into_bits())
                        .with_addr(dest_pdt.as_ptr() as usize)
                ).to_bare();
            }

            dest_pml4[i] = PageMapLevel4Entry::from_bits(src_pml4e.into_bits())
                .with_addr(dest_pdpt.as_ptr() as usize)
                .to_bare();
        }

        PageMap {
            base: NonNull::new(dest_pml4).unwrap(),
        }
    }
}

const fn virt_from_bits_page_aligned(addr: usize) -> VirtAddr {
    addr << PAGE_OFFSET
}

const fn virt_to_bits_page_aligned(addr: VirtAddr) -> usize {
    addr >> PAGE_OFFSET
}


/// Raw PML4E that could be unpresent, so accessing any other fields is UB.
#[bitfield(u64)]
struct PageMapLevel4EntryBare {
    present: bool,
    #[bits(63)]
    _reserved: usize,
}

/// PML4E that is guaranteed to be present.
#[bitfield(u64)]
struct PageMapLevel4Entry {
    #[bits(default = true)]
    _present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    _ignored1: bool,
    _reserved: bool,
    #[bits(4)]
    _ignored2: usize,
    #[bits(40, from = virt_from_bits_page_aligned, into = virt_to_bits_page_aligned)]
    addr: VirtAddr,
    #[bits(11)]
    _ignored3: usize,
    execute_disabled: bool,
}

impl PageMapLevel4Entry {
    fn from_bare(bare: PageMapLevel4EntryBare) -> Option<Self> {
        bare.present().then_some(Self::from_bits(bare.0))
    }

    /// Grab a PML4E from PageMapLevel4EntryBare, overwriting it with a given PML4E if not present. Useful for mapping.
    fn from_bare_or_overwrite(bare: &mut PageMapLevel4EntryBare, pml4e: Self) -> Self {
        Self::from_bare(*bare).unwrap_or_else(|| {
            *bare = pml4e.to_bare();
            pml4e
        })
    }

    fn to_bare(self) -> PageMapLevel4EntryBare {
        PageMapLevel4EntryBare(self.0)
    }

    fn page_dir_ptr_table(&self) -> Option<NonNull<[PageDirectoryPointerEntryBare; 512]>> {
        NonNull::new(self.addr() as *mut _)
    }
}

#[bitfield(u64)]
struct PageDirRef {
    #[bits(default = true)]
    _present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    _ignored1: bool,
    _huge: bool,
    _ignored2: bool,
    #[bits(2)]
    _ignored3: usize,
    reset: bool,
    #[bits(40, from = virt_from_bits_page_aligned, into = virt_to_bits_page_aligned)]
    addr: VirtAddr,
    #[bits(11)]
    _ignored4: usize,
    execute_disabled: bool,
}

impl PageDirRef {
    fn page_dir_table(&self) -> Option<NonNull<[PageDirectoryEntryBare; 512]>> {
        NonNull::new(self.addr() as *mut _)
    }
}

const PAGE_1G_OFFSET: usize = 30;
const PAGE_1G_MASK: usize = (1 << PAGE_1G_OFFSET) - 1;

const fn virt_from_bits_1g_page_aligned(addr: usize) -> VirtAddr {
    addr << PAGE_1G_OFFSET
}

const fn virt_to_bits_1g_page_aligned(addr: VirtAddr) -> usize {
    addr >> PAGE_1G_OFFSET
}

#[bitfield(u64)]
struct Huge1GPageRef {
    #[bits(default = true)]
    _present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    dirty: bool,
    _huge: bool,
    global: bool,
    #[bits(2)]
    _ignored1: usize,
    reset: bool,
    pat: bool,
    #[bits(17)]
    _reserved: usize,
    #[bits(22, from = virt_from_bits_1g_page_aligned, into = virt_to_bits_1g_page_aligned)]
    addr: VirtAddr,
    #[bits(7)]
    _ignored2: usize,
    #[bits(4)]
    pk: usize,
    execute_disabled: bool,
}

#[bitfield(u64)]
struct PageDirectoryPointerEntryBare {
    present: bool,
    #[bits(6)]
    _empty: usize,
    huge: bool,
    #[bits(56)]
    _empty2: usize,
}

#[derive(Debug, Clone, Copy)]
enum PageDirectoryPointerEntry {
    NextLevel(PageDirRef),
    Huge(Huge1GPageRef),
}

impl PageDirectoryPointerEntry {
    fn from_bare(bare: PageDirectoryPointerEntryBare) -> Option<Self> {
        bare.present()
            .then_some(
                if bare.huge() {
                    Self::Huge(Huge1GPageRef::from_bits(bare.0))
                } else {
                    Self::NextLevel(PageDirRef::from_bits(bare.0))
                })
    }

    fn to_bare(self) -> PageDirectoryPointerEntryBare {
        PageDirectoryPointerEntryBare(match self {
            Self::NextLevel(p) => p.0,
            Self::Huge(p) => p.0,
        })
    }

    fn from_bare_or_overwrite(bare: &mut PageDirectoryPointerEntryBare, pdpe: Self) -> Self {
        Self::from_bare(*bare).unwrap_or_else(|| {
            *bare = pdpe.to_bare();
            pdpe
        })
    }
}

#[bitfield(u64)]
struct PageTableRef {
    #[bits(default = true)]
    _present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    _ignored1: bool,
    _huge: bool,
    _ignored2: bool,
    #[bits(2)]
    _ignored3: usize,
    reset: bool,
    #[bits(40, from = virt_from_bits_page_aligned, into = virt_to_bits_page_aligned)]
    addr: VirtAddr,
    #[bits(11)]
    _ignored4: usize,
    execute_disabled: bool,
}

impl PageTableRef {
    fn page_table(&self) -> Option<NonNull<[PageTableEntryBare; 512]>> {
        NonNull::new(self.addr() as *mut _)
    }
}

const PAGE_2M_OFFSET: usize = 21;
const PAGE_2M_MASK: usize = (1 << PAGE_2M_OFFSET) - 1;

const fn virt_from_bits_2m_page_aligned(addr: usize) -> VirtAddr {
    addr << PAGE_2M_OFFSET
}

const fn virt_to_bits_2m_page_aligned(addr: VirtAddr) -> usize {
    addr >> PAGE_2M_OFFSET
}

#[bitfield(u64)]
struct Huge2MPageRef {
    #[bits(default = true)]
    _present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    dirty: bool,
    _huge: bool,
    global: bool,
    #[bits(2)]
    _ignored1: usize,
    reset: bool,
    pat: bool,
    #[bits(8)]
    _reserved: usize,
    #[bits(31, from = virt_from_bits_2m_page_aligned, into = virt_to_bits_2m_page_aligned)]
    addr: VirtAddr,
    #[bits(7)]
    _ignored2: usize,
    #[bits(4)]
    pk: usize,
    execute_disabled: bool,
}

#[bitfield(u64)]
struct PageDirectoryEntryBare {
    present: bool,
    #[bits(6)]
    _empty: usize,
    huge: bool,
    #[bits(56)]
    _empty2: usize,
}

#[derive(Debug, Clone, Copy)]
enum PageDirectoryEntry {
    NextLevel(PageTableRef),
    Huge(Huge2MPageRef),
}

impl PageDirectoryEntry {
    fn from_bare(bare: PageDirectoryEntryBare) -> Option<Self> {
        bare.present().then_some(
            if bare.huge() {
                Self::Huge(Huge2MPageRef::from_bits(bare.0))
            } else {
                Self::NextLevel(PageTableRef::from_bits(bare.0))
            }
        )
    }

    fn to_bare(self) -> PageDirectoryEntryBare {
        PageDirectoryEntryBare(match self {
            Self::NextLevel(p) => p.0,
            Self::Huge(p) => p.0,
        })
    }

    fn from_bare_or_overwrite(bare: &mut PageDirectoryEntryBare, pde: Self) -> Self {
        Self::from_bare(*bare).unwrap_or_else(|| {
            *bare = pde.to_bare();
            pde
        })
    }
}

#[bitfield(u64)]
struct PageTableEntryBare {
    present: bool,
    #[bits(63)]
    _reserved: usize,
}

#[bitfield(u64)]
struct PageTableEntry {
    #[bits(default = true)]
    _present: bool,
    write_enabled: bool,
    user_enabled: bool,
    pwt: bool,
    pcd: bool,
    accessed: bool,
    dirty: bool,
    pat: bool,
    global: bool,
    #[bits(2)]
    _ignored1: usize,
    _r: bool,
    #[bits(40, from = virt_from_bits_page_aligned, into = virt_to_bits_page_aligned)]
    addr: PhysAddr,
    #[bits(7)]
    _ignored2: usize,
    #[bits(4)]
    pk: usize,
    execute_disabled: bool,
}

impl PageTableEntry {
    fn from_bare(bare: PageTableEntryBare) -> Option<Self> {
        bare.present().then_some(Self::from_bits(bare.0))
    }

    fn to_bare(self) -> PageTableEntryBare {
        PageTableEntryBare(self.0)
    }

    fn from_bare_or_overwrite(bare: &mut PageTableEntryBare, pde: Self) -> Self {
        Self::from_bare(*bare).unwrap_or_else(|| {
            *bare = pde.to_bare();
            pde
        })
    }

    fn page(&self) -> Option<NonNull<Page>> {
        NonNull::new(self.addr() as *mut _)
    }
}

unsafe fn la57_enabled() -> bool {
    let mut cr4: usize = 0;

    unsafe {
        asm!(
            "mov {}, cr4",
            out(reg) cr4,
        );
    }

    cr4 & (1 << 12) != 0
}

#[bitfield(u64)]
struct Cr3 {
    #[bits(3)]
    _unused: usize,
    pwt: bool,
    pcd: bool,
    #[bits(7)]
    _unused1: usize,
    #[bits(52, from = virt_from_bits_page_aligned, into = virt_to_bits_page_aligned)]
    pbdr: usize,
}

unsafe fn read_cr3() -> Cr3 {
    let mut cr3: u64 = 0;

    unsafe {
        asm!(
            "mov {}, cr3",
            out(reg) cr3,
        );
    }

    return Cr3::from_bits(cr3)
}

unsafe fn set_cr3(cr3: Cr3) {
    unsafe {
        asm!(
            "mov cr3, {}",
            in(reg) cr3.into_bits(),
        )
    }
}

/// Direct map a physical region of memory to offset.
unsafe fn direct_map_region(
    alloc: &mut boot::mem::EarlyBootAllocator,
    pml4: &mut PageMap,
    offset: usize,
    region: MemoryRegion,
    perms: Perms) -> Result<MemoryRegion, Error>
{
    assert_eq!(offset & PAGE_MASK, 0x0, "cannot map region to an unaligned offset");

    for page in region.iter() {
        let vaddr = unsafe { page.byte_sub(region.base as usize).byte_add(offset) } as usize;
        unsafe { pml4.map_page(alloc, vaddr, page, perms)? };
    }

    Ok(region.rebased(offset as *mut _))
}

#[repr(C)]
pub struct MappedHigherHalf {
    kernel_region: MemoryRegion,
    map: PageMap,
}

pub unsafe fn map_higher_half_kernel(
    alloc: &mut boot::mem::EarlyBootAllocator,
    kernel_region: MemoryRegion) -> Result<MappedHigherHalf, Error>
{
    // kernel does not currently support 5-level paging
    assert!(unsafe { !la57_enabled() });

    let cr3 = unsafe { read_cr3() };

    let map = PageMap {
        // SAFETY: cr3 has to point to a valid, non null page table, or else this code
        // probably couldn't be running right now. 
        base: unsafe { NonNull::new(cr3.pbdr() as *mut _).unwrap_unchecked() },
    };

    let mut nmap = unsafe { map.duplicate(alloc) };
    let nkregion = unsafe { direct_map_region(alloc, &mut nmap, 0xffff800000000000, kernel_region, Perms::WX)? };

    unsafe { set_cr3(cr3.with_pbdr(nmap.base.as_ptr() as usize)) };

    Ok(MappedHigherHalf {
        kernel_region: nkregion,
        map: nmap,
    })
}

unsafe fn create_stack(map: &mut MappedHigherHalf, early: &mut boot::mem::EarlyBootAllocator, npages: usize) -> Result<VirtAddr, Error> {
    let base_addr = unsafe { map.kernel_region.base.add(map.kernel_region.npages) };

    for i in 0..npages {
        let page = unsafe { early.get_page() };
        let page_vaddr = unsafe { base_addr.add(i) } as VirtAddr;

        unsafe { map.map.map_page(early, page_vaddr, page, Perms::WO)? };
    }

    Ok(unsafe { base_addr.add(npages) } as VirtAddr)
}

#[repr(C)]
struct HigherHalfEntry {
    kernel_region: MemoryRegion,
    map: PageMap,
    early: boot::mem::EarlyBootAllocator,
}

pub unsafe fn perform_higher_half_jump(
    mut hh: MappedHigherHalf,
    info: boot::BootData,
    mut early: boot::mem::EarlyBootAllocator) -> !
{
    let hht_addr = unsafe {
        (higher_half_trampoline as *const ())
            .byte_sub(info.kernel_region.base as usize)
            .byte_add(hh.kernel_region.base as usize)
    };

    let trampoline: unsafe extern "C" fn(*const HigherHalfEntry, VirtAddr) -> ! = unsafe {
        core::mem::transmute(hht_addr)
    };

    let Ok(stack) = (unsafe { create_stack(&mut hh, &mut early, 8) }) else {
        panic!("failed to allocate stack");
    };

    let entry = HigherHalfEntry {
        kernel_region: hh.kernel_region,
        map: hh.map,
        early,
    };

    // SAFETY: call into a trampoline which will switch the stack and jump to the real entry.
    unsafe {
        (trampoline)(&raw const entry, stack)
    };
}

#[unsafe(naked)]
unsafe extern "C" fn higher_half_trampoline(payload: *const HigherHalfEntry, stack: VirtAddr) -> ! {
    naked_asm!(
        "mov rsp, rdx",
        "sub rsp, 32",
        "push QWORD PTR 0",
        "jmp {}",
        sym higher_half_entry,
    );
}

unsafe extern "C" fn higher_half_entry(payload: *const HigherHalfEntry) -> ! {
    let payload = unsafe { payload.read() };

    unsafe {
        super::arch_entry(
            ArchEntry {
                map: payload.map,
                kernel_region: payload.kernel_region,
                alloc: payload.early,
            }
        )
    };
}
