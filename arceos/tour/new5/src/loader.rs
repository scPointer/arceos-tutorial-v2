use crate::APP_ENTRY;
use axhal::mem::{phys_to_virt, PAGE_SIZE_4K};
use axhal::paging::MappingFlags;
use axmm::AddrSpace;
use std::fs::File;
use std::io::{self, Read};

// 一个极简的用户程序加载函数
pub fn load_user_app(fname: &str, uspace: &mut AddrSpace) -> io::Result<()> {
    let mut buf = [0u8; 256];
    // 从文件系统中加载 fname 文件中的所有内容。
    // 此处的 std::fs 中的接口实际上来自 axfs 模块。
    load_file(fname, &mut buf)?;

    // 申请在 uspace 中映射一段从 APP_ENTRY 开头的虚拟地址，并给予读/写/执行权限。
    // 此处我们大幅简化了加载流程。
    // 真正实用的加载步骤中，用户程序被看作一个 elf 格式的特殊文件。
    // 其中包含“起始地址在哪”“各段分别设置什么权限”等信息，内核需要根据文件信息完成加载。
    uspace
        .map_alloc(
            APP_ENTRY.into(),
            PAGE_SIZE_4K,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
            true,
        )
        .unwrap();

    // 上面虽然申请了映射，但只要求随便找一块空的地方就行，没有指定映射到哪里。
    // 所以还需要查询页表，才知道具体分配到了哪个物理地址。
    let (paddr, _, _) = uspace
        .page_table()
        .query(APP_ENTRY.into())
        .unwrap_or_else(|_| panic!("Mapping failed for segment: {:#x}", APP_ENTRY));

    ax_println!("paddr of app entry: {:#x}", paddr);

    // 随后，将文件内容复制到该地址上。
    unsafe {
        core::ptr::copy_nonoverlapping(
            buf.as_ptr(),
            phys_to_virt(paddr).as_mut_ptr(),
            PAGE_SIZE_4K,
        );
    }

    Ok(())
}

fn load_file(fname: &str, buf: &mut [u8]) -> io::Result<usize> {
    ax_println!("app: {}", fname);
    let mut file = File::open(fname)?;
    let n = file.read(buf)?;
    Ok(n)
}
