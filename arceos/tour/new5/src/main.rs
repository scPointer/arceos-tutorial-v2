#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;
extern crate alloc;

#[macro_use]
extern crate axlog;

mod task;
mod syscall;
mod loader;

use axstd::io;
use axhal::paging::MappingFlags;
use axhal::arch::UspaceContext;
use axhal::mem::VirtAddr;
use axsync::Mutex;
use alloc::sync::Arc;
use axmm::AddrSpace;
use loader::load_user_app;
use axtask::TaskExtRef;
use axhal::trap::{register_trap_handler, PAGE_FAULT};

const USER_STACK_SIZE: usize = 0x10000;
const KERNEL_STACK_SIZE: usize = 0x40000; // 256 KiB
const APP_ENTRY: usize = 0x1000;

//make disk_img
//make run A=tour/new5 BLK=y
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {

    // 5. 宏内核与用户态程序

    // 在上节实验中，我们提到过机器态(M)/内核态(S)/用户态(U)三种【特权级】
    // qemu虚拟机从机器态启动，初始化完成后会进入内核态，随后内核态进行上节实验介绍的初始化。
    // 但到下一层用户态的流程则有很大不同。

    // 首先，内核会帮助用户态程序准备绝大部分的资源，完成初始化流程。
    // 这使得用户态可以直接运行一个不包含任何初始化的“裸”程序。
    // 当然，用户态程序也可以使用如 cstdio 和 Rust std 等标准库提供的更丰富的接口，这些库会包含一些额外的初始化步骤，但这不是必需的。

    // 下面具体介绍用户程序初始化流程。

    // 5.1 多个地址空间与隔离：页表映射的细节，单双页表

    // 首先，内核需要先创建一个地址空间。
    let mut uspace = axmm::new_user_aspace().unwrap();
    // 地址空间的概念与第二节实验中“虚拟地址”与第三节实验中的“执行流”有关。
    
    // 5.1.1 页表

    // 此处我们假设你对“页表”有简单了解。
    // 如果不是如此，你可以先阅读 tour/new2/src/magic/map.rs 中 check_paddr() 函数的注释，了解虚拟地址查询的过程。
    // 或者参考学习： https://learningos.cn/rCore-Tutorial-Guide-2025S/chapter4/3sv39-implementation-1.html

    // 简而言之，虚拟地址的映射相当于把地址分成“城市号-街道号-门牌号-具体住址”多个部分，每一级存储着下一级页面的地址和访问权限，组成一个树状的结构，称作【页表】。
    // 页表中的内容由内核手动写入，但查询的过程则由硬件自动进行。
    // 整个树状结构的【根】存储在特权寄存器 satp 中。

    // 有时，我们不希望多个执行流如第三节实验那样共享变量，而希望不同的应用程序有各自的隐私。
    // 所以内核会为每个用户程序都准备一个不同的页表。然后在切换不同程序时，直接修改 satp 寄存器。
    // 如此一来，在运行不同程序时，硬件就会在两棵完全不同的树上查询地址，也就将应用程序隔离在了平行世界之中。
    // 我们将这样相互隔离的程序叫做【进程】，而每个进程所属的平行世界叫【地址空间】。

    // 5.1.2 映射区间：MemRegion 和 MemoryArea

    // 除了上述页表机制之外，内核还额外保存一些映射区间的信息。
    use axhal::mem::memory_regions;
    // 区间以 MemRegion 格式存储。我们简单输出这些区间中的前三个：
    for region in memory_regions().take(3) {
        println!("{:#?}", region);
    }
    // 观察输出，可以看到这些区间信息包含起始地址、长度、访问权限和(仅用于调试的)名字。
    // 这些信息是与页表无关、不与硬件交互、单独存储的“备忘录”。
    
    // a. 存储这些信息可以快速响应用户的请求。
    // 页表的映射太多，无法快速处理形如“找到一段长为7MB的空区间并添加映射”的任务。
    // 内核通过成段记录相同类型的区间，可以快速处理这样的任务。

    // b. 映射区间的另一个功能是存储额外信息。
    // 用户可以将磁盘文件映射到内存中，但文件信息过多，无法塞进页表里。此时内核只好翻开“备忘录”来完成操作。

    // 存储额外信息的任务由 MemoryArea 类型完成。
    // 翻阅 Arceos 代码，地址空间的类型为 axmm 模块中的 AddrSpace，其中包含（外部引用模块）中的 MemorySet 类型，内部是 MemoryArea 类型。
    // MemoryArea 与 MemRegion 功能类似，但内部多一个 MappingBackend 存储额外信息。


    // 5.2 宏内核 vs. unikernel 中的程序：文件系统和加载
    // 5.3 初始化整体流程对比：用户栈、“异常中断”和“堆”
    // 5.4 进入用户态：伪装“出生点”
    // 5.5 用户态和内核态的接口：syscall
    // A new address space for user app.
    

    // Load user app binary file into address space.
    if let Err(e) = load_user_app("/sbin/origin", &mut uspace) {
        panic!("Cannot load app! {:?}", e);
    }

    // Init user stack.
    let ustack_top = init_user_stack(&mut uspace, false).unwrap();
    ax_println!("New user address space: {:#x?}", uspace);

    // Let's kick off the user process.
    let user_task = task::spawn_user_task(
        Arc::new(Mutex::new(uspace)),
        UspaceContext::new(APP_ENTRY.into(), ustack_top),
    );

    // Wait for user process to exit ...
    let exit_code = user_task.join();
    ax_println!("monolithic kernel exit [{:?}] normally!", exit_code);
}

fn init_user_stack(uspace: &mut AddrSpace, populating: bool) -> io::Result<VirtAddr> {
    let ustack_top = uspace.end();
    let ustack_vaddr = ustack_top - crate::USER_STACK_SIZE;
    ax_println!(
        "Mapping user stack: {:#x?} -> {:#x?}",
        ustack_vaddr, ustack_top
    );
    uspace.map_alloc(
        ustack_vaddr,
        crate::USER_STACK_SIZE,
        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        populating,
    ).unwrap();
    Ok(ustack_top)
}

#[register_trap_handler(PAGE_FAULT)]
fn handle_page_fault(vaddr: VirtAddr, access_flags: MappingFlags, is_user: bool) -> bool {
    if is_user {
        if !axtask::current()
            .task_ext()
            .aspace
            .lock()
            .handle_page_fault(vaddr, access_flags)
        {
            ax_println!("{}: segmentation fault, exit!", axtask::current().id_name());
            axtask::exit(-1);
        } else {
            ax_println!("{}: handle page fault OK!", axtask::current().id_name());
        }
        true
    } else {
        false
    }
}
