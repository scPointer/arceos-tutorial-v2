use axhal::arch::flush_tlb;
use axhal::mem::phys_to_virt;
use riscv::register::satp;

const FLAGS_MASK: usize = (1 << 10) - 1;
#[repr(transparent)]
struct PTE(usize);

impl PTE {
    pub fn get_paddr(&self) -> usize {
        (self.0 & !FLAGS_MASK) << 2
    }
    pub fn set_paddr(&mut self, addr: usize) {
        *self = PTE(((addr >> 2) & !FLAGS_MASK) | (self.0 & FLAGS_MASK));
    }
}

struct Page {
    entries: [PTE; 512],
}

impl Page {
    pub fn as_page<'a>(addr: usize) -> &'a mut Page {
        unsafe { &mut *(addr as *mut Page) }
    }
    #[allow(unused)]
    pub fn print_non_zero_entries(&self) {
        for (i, pte) in self.entries.iter().enumerate() {
            if pte.0 != 0 {
                println!("pte[{}]: {:x}", i, pte.0);
            }
        }
    }
}

pub fn map(src: usize, dst: usize) {
    let mut page_addr: usize = satp::read().ppn() << 12;

    for index in [(src >> 30) & 0x1ff, (src >> 21) & 0x1ff] {
        page_addr = phys_to_virt(page_addr.into()).as_usize();
        let page = Page::as_page(page_addr);
        page_addr = page.entries[index].get_paddr();
        //println!("page_addr: {:x}, index: {}, pte: {:x}", page_addr, index, page.entries[index].0);
    }

    page_addr = phys_to_virt(page_addr.into()).as_usize();
    let src_page = Page::as_page(page_addr);
    src_page.entries[(src >> 12) & 0x1ff].set_paddr(dst);

    flush_tlb(Some(src.into()));
}

pub fn check_paddr(src: usize) -> usize {
    let mut page_addr: usize = satp::read().ppn() << 12;

    for index in [
        (src >> 30) & 0x1ff,
        (src >> 21) & 0x1ff,
        (src >> 12) & 0x1ff,
    ] {
        page_addr = phys_to_virt(page_addr.into()).as_usize();
        let page = Page::as_page(page_addr);
        page_addr = page.entries[index].get_paddr();
    }

    page_addr
}
