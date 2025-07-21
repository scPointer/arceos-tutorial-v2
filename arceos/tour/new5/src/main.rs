#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

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

//make run A=tour/new5 BLK=y
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {

    // 5. 宏内核与用户态程序

    // 在上节实验中，我们提到过机器态(M)/内核态(S)/用户态(U)三种【特权级】
    // qemu虚拟机从机器态启动，初始化完成后会进入内核态，随后内核态进行上节实验介绍的初始化。
    // 但到下一层用户态的流程则有很大不同。

    // 首先，内核会帮助用户态程序准备绝大部分的资源，完成初始化流程。这使得用户态可以直接运行一个不包含任何初始化的“裸”程序。当然，用户态程序也可以使用如 cstdio 和 Rust std 等标准库提供的更丰富的接口，这些库会包含一些额外的初始化步骤，但这不是必需的。

    // 5.1 多个地址空间与隔离：页表映射的细节，单双页表
    // 5.2 宏内核 vs. unikernel 中的程序：文件系统和加载
    // 5.3 初始化整体流程对比：用户栈、“异常中断”和“堆”
    // 5.4 进入用户态：伪装“出生点”
    // 5.5 用户态和内核态的接口：syscall
    // A new address space for user app.
    let mut uspace = axmm::new_user_aspace().unwrap();

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
