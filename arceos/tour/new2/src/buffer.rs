//! 一个大小为 4096B 的 buffer

// 调用 axalloc 模块提供的内存分配器
use axalloc::global_allocator;
// phys 和 virt 分别是物理地址和虚拟地址, phys_to_virt 和 virt_to_phys 两个函数完成它们之间的转换。
// 注意，经过 ./magic/map.rs 中修改过的地址不再适用这两个函数。
// 在本实验中，暂不介绍这它们的实际机制。
// 只需要知道：在软件程序中访问一个虚拟地址时，硬件实际访问的是物理地址
use axhal::mem::{phys_to_virt, virt_to_phys, PAGE_SIZE_4K};

/// 一个大小为 4096B(4KB)，并以 4096B 为单位对齐的 buffer
pub struct Buffer<'a> {
    pub paddr: usize,
    pub data: &'a mut [u8; PAGE_SIZE_4K],
}

impl Buffer<'_> {
    /// 创建新 buffer
    pub fn new() -> Self {
        // 直接向全局分配器申请一个页(4096B)的内存
        // 这对应了通常用户态程序的“堆”
        let vaddr = global_allocator().alloc_pages(1, PAGE_SIZE_4K).unwrap();
        let paddr = virt_to_phys(vaddr.into()).as_usize();
        let data = unsafe { &mut *(vaddr as *mut [u8; PAGE_SIZE_4K]) };
        Buffer { paddr, data }
    }

    /// 写入 data，最多 4096B
    pub fn set_data(&mut self, data: &str) {
        assert!(data.len() <= PAGE_SIZE_4K);
        self.data[..data.len()].copy_from_slice(data.as_bytes());
    }

    /// 读取数据
    pub fn get_data(&self) -> &str {
        // 找到第一个为 0 的位置，作为字符串结束的标志
        let len = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(PAGE_SIZE_4K);
        unsafe { core::str::from_utf8_unchecked(&self.data[..len]) }
    }
}

impl Drop for Buffer<'_> {
    /// 当 Buffer被析构时，释放申请的内存
    fn drop(&mut self) {
        global_allocator().dealloc_pages(phys_to_virt(self.paddr.into()).as_usize(), 1);
    }
}
