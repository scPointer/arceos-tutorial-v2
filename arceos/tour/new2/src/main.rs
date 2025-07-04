#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

mod magic;
mod buffer;

#[repr(align(512))]
struct Buffer([u8; 512]);

//make run A=tour/new2
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {

    // 2. 通过特权寄存器直接和硬件交互

    // 如果说调用下层接口还只是函数调用的逻辑：给接口输入参数，之后拿到返回值。
    // 那么内核和硬件的直接交互就是魔法了。
    // 例如硬件可以施展以下几种常用魔法：

    // a. 当发生访问空指针之类的“异常”时，自动跳转到某个没有任何人调用的函数，并记录相关信息。
    let mut old_trap: usize = 0;
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        use riscv::register::stvec;
        extern "C" { fn example_trap(); }
        core::arch::asm!("csrrw {rd}, stvec, {rs}", rd = out(reg) old_trap, rs = in(reg) example_trap as usize);
    }

    // 上面大括号中的代码中，前两行都只是在引入符号，
    // 第三行表示保存 stvec 的旧值，将 example_trap 这个函数的地址写入 stvec。
    // stvec 是 RISC-V 硬件上的寄存器。发生异常时，硬件会自动跳转到内部存的值。
    // 而 example_trap 写在 ./magic/trap.S 中，它的内部会调用 main.rs 中 example_trap_handler 函数

    // 下面，我们构造一个指向地址 1 的指针
    let p:*const u64 = 1 as *const u64;
    // 然后尝试读取这个地址的值。
    // 这里的 read_volatile 是为了保证该行代码触发。否则，编译器发现 val 变量没有被实际使用，会优化掉这一行代码。
    let _val = unsafe { core::ptr::read_volatile(p) };

    // 如果在用户态运行上述代码，本程序将会报错退出。
    // 但当我们在内核使用魔法后，它到 example_trap_handler 中转了一圈后又重新回到了此处
    println!("Welcome back");

    // 恢复 stvec 的旧值
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {        
        use riscv::register::stvec;
        if old_trap != 0 {
            stvec::write(old_trap, stvec::TrapMode::Direct);
        }
    }

    // 在 x86, arm 等其他架构上，也有类似的机制，但流程略有不同。
    // 你【不需要】理解 ./magic 下的细节。
    // 本项目已经封装了类似本实验的内容，你可以直接使用在 axhal/trap.rs 中封装好的 IRQ, PAGE_FAULT, SYSCALL
    
    // EXERCISE 1：
    // 1. 将上面操作 stvec 的两行代码注释掉，重新运行，观察执行后的现象。
    // 2. 【保持上一题的状态】，使用 axhal/trap.rs 中的 PAGE_FAULT 宏编写一个函数，打印一行输出（可参考example_trap_handler）
    // TIPS：可以在项目中搜索 PAGE_FAULT 宏如何使用
    // TIPS：在本题中，你【无法】回到本函数中打印 Welcome back，只能额外打印输出后退出。这是正常的。



    // EXERCISE 1 END

    // b. 让相同地址的指针访问到不同数据，让不同地址的指针访问到相同的数据，且可以动态调整。

    // 这些功能是由硬件完成的，不会遵循函数调用后返回之类的逻辑。
    // 编程语言的编译器通常不参与这些功能，也无法检查其中的错误。
    // 所以内核需要小心地使用/包装这些功能，给编译器营造一个世界上没有魔法的假象。
    // 当然，这些功能也和硬件架构相关。我们可以从 axhal 中获取一部分接口。

    use riscv::register::satp;
    let satp:usize = satp::read().ppn(); // 读取当前的页表根地址
    println!("Current satp: {:x}", satp);
    
    use buffer::Buffer;
    let mut page_a:Buffer = Buffer::new();
    let mut page_b:Buffer = Buffer::new();

    println!("page_a vaddr: {:#x}", page_a.vaddr);
    println!("page_b vaddr: {:#x}", page_b.vaddr);
    page_a.set_data("Hello from A.");
    println!("page_a data: {:#}", page_a.get_data());
    page_b.set_data("Hello from B. ".repeat(20).as_str());
    println!("page_b data: {:#}", page_b.get_data());
    
    use magic::map;

    println!("\nmap a->b");
    map(page_a.data.as_ptr() as usize, page_b.data.as_ptr() as usize);
    
    println!("page_a data: {:#}", page_a.get_data());
    println!("page_a vaddr: {:#x}", page_a.data.as_ptr() as usize);
    println!("page_b vaddr: {:#x}", page_b.data.as_ptr() as usize);

    println!("\nmap b->a");
    map(page_b.data.as_ptr() as usize, page_a.data.as_ptr() as usize);
    
    println!("page_b data: {:#}", page_b.get_data());
    println!("page_a vaddr: {:#x}", page_a.data.as_ptr() as usize);
    println!("page_b vaddr: {:#x}", page_b.data.as_ptr() as usize);

    // to be continued...
    let c: usize = 9usize;
    println!("c = {}", c);
    println!("now");
    
    for r in axhal::mem::memory_regions() {
        println!(
            "Memory region {:?}", r);
    }
    //此时又有细心的同学注意到

    axhal::misc::terminate();
}

use riscv::register::scause::{self, Exception, Trap, Interrupt};
use riscv::register::sie;

use crate::magic::map;

#[no_mangle]
fn example_trap_handler(tf: &mut magic::TrapFrame) {
    let scause = scause::read();
    match scause.cause() {
        Trap::Exception(Exception::LoadPageFault) => {
            println!("LoadPageFault at {:#x}\n", tf.sepc);
            tf.sepc += 4; // Skip the faulting instruction
        },
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            unsafe { sie::clear_stimer() };
        },
        _ => {
            panic!(
                "Unhandled trap {:?} @ {:#x}:\n{:#x?}",
                scause.cause(),
                tf.sepc,
                tf
            );
        }
    }
}
