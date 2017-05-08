use core::ptr::Unique;
use super::frame::{Frame, FrameAllocator};
use super::paging::*;

pub struct Mapper {
    top: Unique<Table<PML4T>>
}

use core::ops::{Deref, DerefMut};
impl Deref for Mapper {
    type Target = Table<PML4T>;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl DerefMut for Mapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl Mapper {
    /// pointer to top level table virtual address, only if table is recursive-mapped.
    pub fn new() -> Mapper {
        Mapper {
            top: unsafe { Unique::new(0xffffffff_fffff000 as *mut _) }
        }
    } 

    pub fn get(&self) -> &Table<PML4T> {
        unsafe { self.top.get() }
    }

    pub fn get_mut(&mut self) -> &mut Table<PML4T> {
        unsafe { self.top.get_mut() }
    }

    pub fn translate(&self, vaddr: VirtualAddress) -> Option<PhysicalAddress> {
        vaddr.validate();

        let p3 = self.next_level_table(vaddr.pml4t_index());
        let offset = vaddr.offset();

        /// return frame for huge page
        let huge_page = || -> Option<Frame> {
            p3.and_then(|p3| {
                let entry = &p3[vaddr.pdpt_index()];
                if let Some(frame) = entry.pointed_frame() {
                    //1G-page 
                    if entry.flags().contains(HUGE_PAGE) {
                        assert!(frame.number % (ENTRY_COUNT * ENTRY_COUNT) == 0);
                        return Some(Frame {
                            number: frame.number + vaddr.pdt_index() * ENTRY_COUNT + vaddr.pt_index() 
                        });
                    }
                }

                if let Some(p2) = p3.next_level_table(vaddr.pdpt_index()) {
                    let entry = &p2[vaddr.pdt_index()];
                    if let Some(frame) = entry.pointed_frame() {
                        //2M-page
                        if entry.flags().contains(HUGE_PAGE) {
                            assert!(frame.number % ENTRY_COUNT == 0);
                            return Some(Frame {
                                number: frame.number + vaddr.pt_index()
                            });
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_level_table(vaddr.pdpt_index()))
            .and_then(|p2| p2.next_level_table(vaddr.pdt_index()))
            .and_then(|p1| p1[vaddr.pt_index()].pointed_frame())
            .or_else(huge_page)
            .map(|frame| frame.start_address() + offset)
    }

    //FIXME: need to check if frame has been used
    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A) 
        where A: FrameAllocator {
        let vaddr = page.start_address() as VirtualAddress;

        let pdpt = self.next_level_table_or_create(vaddr.pml4t_index(), allocator);
        let pdt = pdpt.next_level_table_or_create(vaddr.pdpt_index(), allocator);
        let pt = pdt.next_level_table_or_create(vaddr.pdt_index(), allocator);

        assert!(pt[vaddr.pt_index()].is_unused());
        pt[vaddr.pt_index()].set(frame, flags | PRESENT);
    }


    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A) 
        where A: FrameAllocator {
        let frame = allocator.alloc_frame().expect("not more free frame available");
        self.map_to(page, frame, flags, allocator)
    }

    pub fn identity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocator {
        let page = Page::from_vaddress(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }

    //TODO: support huge page
    pub fn unmap<A>(&mut self, page: Page, allocator: &mut A) where A: FrameAllocator {
        let vaddr = page.start_address() as VirtualAddress;
        assert!(self.translate(vaddr).is_some(), "vaddr {:#x} doest exist in mapping", vaddr);

        let p3 = self.next_level_table_mut(vaddr.pml4t_index());

        let huge_page = || {None};

        p3.and_then(|p3| p3.next_level_table_mut(vaddr.pdpt_index()))
            .and_then(|p2| p2.next_level_table_mut(vaddr.pdt_index()))
            .and_then(|p1| {
                assert!(!p1[vaddr.pt_index()].is_unused());
                let frame = p1[vaddr.pt_index()].pointed_frame().unwrap();
                p1[vaddr.pt_index()].set_unused();

                ::kern::arch::cpu::tlb_flush(vaddr);
                //TODO: free pdpt, pdt, pt tables when empty
                //allocator.dealloc_frame(frame);
                Some(())
            })
            .or_else(huge_page);
    }
}

