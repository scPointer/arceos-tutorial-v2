use riscv::register::satp;
use axhal::mem::{phys_to_virt, virt_to_phys, PAGE_SIZE_4K}; 
use axhal::arch::flush_tlb;

//const PAGE_MASK: usize = PAGE_SIZE_4K - 1;
const FLAGS_MASK: u64 = (1 << 10) - 1;
#[repr(transparent)]
struct PTE(u64);

impl PTE {
    pub fn get_paddr(&self) -> u64 {
        (self.0 & !FLAGS_MASK) << 2
    }
    pub fn set_paddr(&mut self, addr: u64) {
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
    pub fn print_non_zero_entries(&self) {
        for (i, pte) in self.entries.iter().enumerate() {
            if pte.0 != 0 {
                println!("pte[{}]: {:x}", i, pte.0);
            }
        }
    }
}

pub fn map(src: usize, dst: usize) {
    let root:usize = satp::read().ppn() << 12;
    let vroot = phys_to_virt(root.into()).as_usize();
    
    println!("root: {:x}", root);
    println!("vroot: {:x}", vroot);
    
    let mut page_addr:usize = vroot;
    
    
    /*
    for index in [(dst >> 30) & 0x1ff, (dst >> 21) & 0x1ff, (dst >> 12) & 0x1ff] {
        let page = Page::as_page(page_addr);
        page_addr = page.entries[index].get_paddr() as usize;
        page_addr = phys_to_virt(page_addr.into()).as_usize();
        println!("index {}, page_addr: {:x}", index, page_addr);
    }

    let dst_paddr = virt_to_phys(page_addr.into()).as_usize();
     */
    let dst_paddr = virt_to_phys(dst.into()).as_usize();
    page_addr = vroot;
    for index in [(src >> 30) & 0x1ff, (src >> 21) & 0x1ff] {
        let page = Page::as_page(page_addr);
        page_addr = page.entries[index].get_paddr() as usize;
        page_addr = phys_to_virt(page_addr.into()).as_usize();
        println!("index {}, page_addr: {:x}", index, page_addr);
    }

    let mut src_page = Page::as_page(page_addr);
    src_page.entries[(src >> 12) & 0x1ff].set_paddr(dst_paddr as u64);

    flush_tlb(Some(src.into()));
}