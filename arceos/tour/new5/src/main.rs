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

use core::{arch::asm, ffi::c_char, usize};

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

//（以下在 Arceos 根目录操作）
// make disk_img
// make payload
// ./update_disk.sh payload/origin/origin
// make run A=tour/new5 BLK=y
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

    // 5.1 多地址空间与进程隔离

    // 在第三节实验中，多个执行流可以共享变量；在第四节实验未提及的多核启动部分，多个CPU也可以共同访问整个内存。
    // 这在内核中没问题，但到了用户态一层，我们还是希望各个程序有自己的隐私，避免跨应用窃取数据。
    // 这就需要将不同的程序隔离开。

    // 首先，内核需要先创建一个地址空间。
    let mut uspace = axmm::new_user_aspace().unwrap();
    // 地址空间的概念与第二节实验中“虚拟地址”与第三节实验中的“执行流”有关。
    
    // 5.1.1 页表

    // 此处假设你对“页表”有简单了解。
    // 如果不是如此，可以先阅读 tour/new2/src/magic/map.rs 中 check_paddr() 函数的注释，了解虚拟地址查询的过程。
    // 或者参考学习： https://learningos.cn/rCore-Tutorial-Guide-2025S/chapter4/3sv39-implementation-1.html

    // 简而言之，虚拟地址的映射相当于把地址分成“城市号-街道号-门牌号-具体住址”多个部分，每一级存储着下一级页面的地址和访问权限，组成一个树状的结构，称作【页表】。
    // 页表中的内容由内核手动写入，但查询的过程则由硬件自动进行。
    // 整个树状结构的根存储在特权寄存器 satp 中。

    // 为了隔离程序，内核会为每个用户程序都准备一个不同的页表。
    // 然后在切换不同程序时，直接修改 satp 寄存器。
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

    // 5.2.1 文件系统与加载

    // 在此前所有实验中，我们的实验代码都可以直接引用 Arceos 内部模块的功能和实现。
    // 这是因为它们全都被编译成第四节实验提到的二进制文件，作为整体交给 qemu 虚拟机运行。
    
    // 但用户程序并不和内核一同编译。
    // 具体来说，用户程序编译后被存储在虚拟的硬盘设备中，内核使用【文件系统】模块与硬盘设备交互，将其中的用户程序【加载】到内存中运行。

    // 例如本节实验需要运行以下四条而非一条指令：
    //      生成一个硬盘镜像
    //      make disk_img
    //      然后编译在 payload 目录下的用户程序。它们没有引用任何 Arceos 的内部模块，是完全独立的。
    //      make payload
    //      再通过脚本将用户文件放进硬盘中
    //      ./update_disk.sh payload/origin/origin
    //      最后在启动时加入 BLK 选项，让虚拟机加载镜像
    //      make run A=tour/new5 BLK=y

    // 内核启动之后，我们从文件系统中加载对应名字的文件
    if let Err(e) = load_user_app("/sbin/origin", &mut uspace) {
        panic!("Cannot load app! {:?}", e);
    }

    // 上述分开编译的好处是，你可以使用 Rust、C或者任何其他语言编译的二进制程序，内核并不关心它们的来源。
    // 但与之相对的，用户程序无法直接使用任何内核功能。
    // 这样的用户程序如何与内核交互呢？

    // 5.2.2 跨特权级的接口：ecall指令异常

    // 在第一节实验中提到，内核可以调用下层 OpenSBI 的接口；在第四节实验中提到，内核启动后，无需任何初始化，即可立即使用这些接口。
    // 事实上，“使用”这些接口是通过特殊硬件指令实现的。在 RISC-V 架构下，这条指令是 ecall。

    unsafe {
        let mut return_val: usize = usize::MAX;
        // 以 getchar 为例，我们循环执行 ecall 指令试图读取一个字符
        println!("enter any key:");
        while return_val == usize::MAX {
            asm!("ecall",
            // 执行前，内核会在约定好的寄存器中填入参数，代表请求类型。
                in("a7") 2,
            // 硬件会判断异常的类型，将其交给机器态的 OpenSBI 处理（而非内核的 trap 函数）。
            // OpenSBI 读取并识别请求参数后，完成操作，并将返回值写入约定好的寄存器中。
                out("a0") return_val,
            );
        }
        // 之后内核恢复执行，从寄存器中读出返回值——例如 getchar 从键盘读取的字符。
        println!("You entered:{}({})", return_val, return_val as u8 as char);

    }

    // 用户与内核的交互也是如此。此时只是换成用户调用 ecall 指令，内核来处理并返回用户的请求。
    // 例如本实验的用户程序 payload/origin/src/main.rs 其实仅包含如下代码：
    // addi sp, sp, -4
    // sw a0, (sp)
    // li a7, 93
    // ecall
    //
    // 其本质就是将参数填入特定寄存器中，然后调用 ecall 指令。

    // 第一节实验给出的文档介绍了 OpenSBI 各个接口参数的定义，即输入什么数代表什么功能，它是和 RISC-V 这个架构绑定的。
    // 而在用户与内核这一层，使用的是一组被称为POSIX 【syscall】的接口，它目前是与 Linux 这个平台绑定的。
    // 我们稍后还会详细介绍 syscall 这层接口。

    // EXERCISE 1:
    // 1. ecall 指令并不直接写在 Arceos 的代码里，而是在依赖库中被调用。请利用上一节实验提到的反汇编方法，在内核二进制文件中找到 ecall。ecall 出现在什么函数中？为什么会是它们？
    // EXERCISE 1 END


    // 5.3 初始化整体流程对比：用户栈、“异常中断”和“堆”
    // 5.4 进入用户态：伪装“出生点”
    // 5.5 用户态和内核态的接口：syscall
    // A new address space for user app.
    



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
