use axalloc::global_allocator;
use axhal::mem::PAGE_SIZE_4K;

pub struct Buffer<'a> {
    pub vaddr: usize,
    //pub len: usize,
    pub data: &'a mut [u8; PAGE_SIZE_4K],
}

impl Buffer<'_> {
    pub fn new() -> Self {
        let vaddr = global_allocator().alloc_pages(1, PAGE_SIZE_4K).unwrap();
        //let len = 0usize;
        let data = unsafe { &mut *(vaddr as *mut [u8; PAGE_SIZE_4K]) };
        Buffer { vaddr, data }
    }
    
    pub fn set_data(&mut self, data: &str) {
        assert!(data.len() <= PAGE_SIZE_4K);
        self.data[..data.len()].copy_from_slice(data.as_bytes());
        //self.len = data.len();
    }

    pub fn get_data(&self) -> &str {
        let len = self.data.iter().position(|&b| b == 0).unwrap_or(PAGE_SIZE_4K);
        unsafe { core::str::from_utf8_unchecked(&self.data[..len]) }
    }
}

impl Drop for Buffer<'_> {
    fn drop(&mut self) {
        global_allocator().dealloc_pages(self.vaddr, 1);
    }
}

