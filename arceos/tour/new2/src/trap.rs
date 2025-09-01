//! 演示硬件异常中断机制

// （你可能需要自学什么叫通用寄存器，再了解下面的“特权寄存器”）
// sie, stvec, scause 这些是 RISC-V 架构的特权寄存器。
// 我们不详述这些寄存器的细节定义，但你可能需要了解“特权寄存器”这一概念。
// 它们和通用寄存器不同，不仅是硬件上存储的数据，还通过特殊规则直接影响硬件行为
// 这些寄存器的规定和行为十分复杂，是由 RISC-V 指令集规定的。
// 如感兴趣，你可以在这里获取最新的指令集 https://github.com/riscv/riscv-isa-manual/releases/
//
// 其他架构也有类似的规范，提供类似的功能。
// axhal 模块统一了这些类似的功能，因此在开发时只需调用 axhal 即可。
use riscv::register::scause::{self, Exception, Interrupt, Trap};
use riscv::register::sie;

use crate::magic::TrapFrame;

/// 在 example_trap 环境中执行外部提供的函数 f
pub fn set_trap() {
    // [rust] unsafe 是 Rust 的特殊关键字。
    // 声明函数或 trait 时添加此关键字表示 Rust 的编译器检查无法保护此处的安全性，相当于“免责声明”；
    // 另一方面，unsafe 函数或 trait 的调用者(此处的内核)必须在调用时添加 unsafe 关键字，表示“已了解该风险”。
    // 在下面这个块中，core::arch::asm 宏的编写者决定将其定义为 unsafe，
    // 因为我们完全可以在此处插入其他随意修改寄存器、跳转的指令，导致未知的后果。
    // 而我们在此处添加的 unsafe 则表示我们知道自己在做什么，并为此负责。
    //
    // 通常的 Rust 编程中不建议使用 unsafe 块，但它在内核中确是不可避免的。
    // 这是因为，无论是下层硬件指令还是上层应对用户程序的接口，都有大量规范写在文档文件里，
    // 而 Rust 并不知道某条指令是什么含义，或者某个整数代表什么指针。
    // 当然，unsafe 还是尽量越少越好。这就是为什么我们有 axstd / axhal 这些模块，
    // 它们直接操作“危险的”unsafe接口，生成安全的接口，
    // 这样其他模块的编写者就可以只使用安全接口，不必担心 unsafe 块引发的错误。
    unsafe {
        // 引入一个函数符号 example_trap，它是一个写在 ./magic/trap.S 汇编里的函数
        // 它在内部会调用本文件中的 example_trap_handler 函数
        // [rust] extern "C" 表示引用一个符合 C 语言规范的符号，使得 Rust 可以调用 C 的函数
        // 也可以在定义 Rust 函数时使用 extern "C" 和后面介绍的 [no_mangle]，让 C 调用 Rust 的函数
        // 类似地，也可以在类型定义前加入 #[repr(C)] 使得类型排布与 C 相同，
        // 避免 Rust 编译器重排结构体成员变量等等排布不同的情况
        extern "C" {
            fn example_trap();
        }
        // 这条汇编指令会将 example_trap 函数的地址写入特权寄存器 stvec 中。
        // stvec 是 RISC-V 硬件上的寄存器。发生异常时，硬件会自动跳转到内部存的值。
        core::arch::asm!("csrw stvec, {rs}", rs = in(reg) example_trap);
    }
}

// [rust] 为了支持不同依赖库中的同名函数，以及同个依赖库的多个版本中的同名函数，
// Rust 编译器会修改函数名，以保证函数调用正确。
// 但此处这个函数需要被 trap.S 调用，其中的函数名 example_trap_handler 是写死的。
// 所以我们需要加上 no_mangle 关键字以禁止编译器修改。
#[no_mangle]
/// 在 set_trap 中，发生的异常会被跳转到这里处理
pub fn example_trap_handler(tf: &mut TrapFrame) {
    let scause = scause::read();
    match scause.cause() {
        Trap::Exception(Exception::LoadPageFault) => {
            // 读取非法页。通常来说这应该使用户程序终止，但在本实验中，我们忽略并跳过此异常。
            // 见 main.rs
            println!("LoadPageFault at {:#x}\n", tf.sepc);
            tf.sepc += 4; // 跳过引发异常的指令，避免反复触发异常
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            unsafe { sie::clear_stimer() };
        }
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
