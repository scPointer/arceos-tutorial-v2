use alloc::sync::Arc;
use core::arch::asm;

use axhal::arch::{TrapFrame, UspaceContext};
use axmm::AddrSpace;
use axsync::Mutex;
use axtask::{AxTaskRef, TaskExtRef, TaskInner};

// 5.ex TaskExt

// 本段介绍 Arceos 提供的 TaskExt 功能。它其实算第三节实验的一个后续。

/// Arceos 中执行流的可变参数由 axtask/src/task.rs:TaskInner 维护。
/// TaskInner 允许通过扩展的方式添加自定义的内容。
pub struct TaskExt {
    /// 例如加入用户程序的上下文
    pub uctx: UspaceContext,
    /// 以及地址空间等等
    pub aspace: Arc<Mutex<AddrSpace>>,
}
// 这些内容可以在执行流运行时通过 axtask::current().task_ext() 调用。
// TaskExt 就像“我的文档”或者 /home 目录，用于获取正在运行的程序的信息。
// 当然，也可以添加数组或哈希表作为全局变量，实现同样的功能，但这会使内核比较杂乱，也不方便回收资源。

impl TaskExt {
    pub const fn new(uctx: UspaceContext, aspace: Arc<Mutex<AddrSpace>>) -> Self {
        Self { uctx, aspace }
    }
}

// 定义类型之后，需要通过 axtask 提供的宏进行注册。
axtask::def_task_ext!(TaskExt);

// 创建并启动用户程序
pub fn spawn_user_task(aspace: Arc<Mutex<AddrSpace>>, uctx: UspaceContext) -> AxTaskRef {
    // 先定义一个执行流。注意，它此时还未被 spawn，所以不会立即启动。
    let mut task = TaskInner::new(
        || {
            // 获取目前运行时的执行流
            // 此后使用的 curr 中的量都并非定义 task 时从外部捕获的变量，
            // 而是执行流实际运行到这一行代码时，其内部的变量。
            // 因此这个函数不需要像第三节实验那样，小心地处理外部变量与 clone 的关系。
            let curr = axtask::current();
            // 获取当期执行流的内核栈顶
            // 它并不是第四节实验中整个内核初始化时的栈，而是创建新执行流时，属于执行流的资源
            let kstack_top = curr.kernel_stack_top().unwrap();
            unsafe {
                // 调用下面的函数，其中 task_ext() 中的量是在下面几行 task.init_task_ext 时传入的。
                enter_user(
                    // 其中包含用户的上下文，或者说“存档点”信息
                    &curr.task_ext().uctx as *const _ as usize,
                    // 以及用户程序第一行代码的位置，
                    curr.task_ext().uctx.get_ip(),
                    // 还有内核栈顶位置
                    kstack_top.as_usize(),
                )
            }
        },
        "userboot".into(),
        crate::KERNEL_STACK_SIZE,
    );
    // 随后，给这个 task 设置页表和 task_ext。
    // 当 axtask 模块切换不同执行流时，会自动检测并切换页表。
    task.ctx_mut()
        .set_page_table_root(aspace.lock().page_table_root());
    task.init_task_ext(TaskExt::new(uctx, aspace));
    // 最后，我们放出这个装着用户程序的执行流，开始运行
    axtask::spawn_task(task)
}

#[inline(never)]
#[no_mangle]
pub unsafe fn enter_user(user_ctx: usize, entry: usize, kstack_top: usize) -> ! {
    use riscv::register::{sepc, sscratch, sstatus};

    // 一些第二节实验中的硬件魔法扩展。
    // 简单来说，此处暂时关闭内核中断功能
    sstatus::clear_sie();
    // 将内核栈顶写入特殊寄存器（它在之后讲解 syscall 时有用）
    sscratch::write(kstack_top);
    // 然后将用户第一条指令的地址写入“出错地址”sepc
    sepc::write(entry);
    // 后续从用户返回时，用户的上下文（TrapFrame）会被存在内核栈顶，即 kstack_top 处。
    // 而内核自己的信息则存在用户上下文的下面，即 kernel_trap_addr 处。
    let kernel_trap_addr = kstack_top - core::mem::size_of::<TrapFrame>();
    asm!("
        mv      sp, {tf}
        
        STR     gp, {kernel_trap_addr}, 2
        LDR     gp, sp, 2

        STR     tp, {kernel_trap_addr}, 3
        LDR     tp, sp, 3

        LDR     t0, sp, 32
        csrw    sstatus, t0
        POP_GENERAL_REGS
        LDR     sp, sp, 1
        sret",
        tf = in(reg) user_ctx,
        kernel_trap_addr = in(reg) kernel_trap_addr,
        options(noreturn),
    )
    // 除了涉及 kernel_trap_addr 的几条保存之外，上面执行的汇编代码相当于第二节实验中 magic/trap.S 的后半部分。
    // 即，将准备好的上下文真正装入寄存器中。
    // 最后再用一条 sret 指令让硬件“返回”用户态执行。
    // 接下来请回到 main.rs 中继续阅读。
}
