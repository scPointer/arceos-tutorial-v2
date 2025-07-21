//! 5.5 系统调用
//!
//! 系统调用(syscall)是一组规定用户程序如何与内核交互的接口。
//! syscall 本身是 POSIX 规范的一部分，
//! 但受到 Linux 影响，导致接口演化到现在包含许多Linux 的内容和特性，其他平台运行时需要一定程度上“模拟”这些特性。
//! 不过本实验暂不涉及这些复杂的特性。
//! 你可以在 man7 网站查询到 syscall 接口的定义：https://www.man7.org/linux/man-pages/man2/syscall.2.html
#![allow(dead_code)]

use axerrno::LinuxError;
use axhal::arch::TrapFrame;
use axhal::trap::{register_trap_handler, SYSCALL};
use axtask::TaskExtRef;

// 5.5.1 系统调用的参数定义

// 规范只规定了系统调用的类型、名字、参数信息，
// 至于具体执行时使用哪条指令、什么寄存器里存什么数对应哪个系统调用，则取决于硬件架构。

// 以 RISC-V 为例，系统调用使用 ecall 指令触发，系统调用编号存在 a7 寄存器，而参数依次存在 a0-a5 寄存器中，调用完成后，需要将结果保存至 a0 寄存器后返回。
// 系统调用编号与名称的对应关系一般定义在系统头文件中，但也可在网上查询。
// RISC-V 的系统调用编号可以查询 https://jborza.com/post/2021-05-11-riscv-linux-syscalls/
// 其他一些常用架构可以查 https://syscall.sh/
const SYS_EXIT: usize = 93;
const SYS_WRITE: usize = 64;

// 5.5.2 系统调用如何触发
// 在 5.2.2 提到，系统调用(syscall)是由 ecall 触发的异常指令。
// 当前实验中，没有如第二节实验那样手动构造一个函数来处理异常，而是直接使用第四节实验初始化时 axhal 提供的异常处理函数。
// 也即，这个异常会被发到 modules/axhal/src/arch/riscv/trap.rs 的 riscv_trap_handler 处理。请简单阅读这个函数。
// 之后，异常类型为 Trap::Exception(E::UserEnvCall) 的系统调用会被发给 handle_syscall 函数处理。
// 经由 modules/axhal/src/trap.rs 的同名函数，通过宏定义跳转到下面的函数执行
#[register_trap_handler(SYSCALL)]
fn handle_syscall(tf: &TrapFrame, syscall_num: usize) -> isize {
    // 系统调用最多包含6个参数，
    // 如果编号对应的系统调用不足6个参数，则按a0,a1,a2,a3,a4,a5的顺序依次填充
    println!(
        "handle_syscall {}({},{},{},{},{},{})",
        syscall_num,
        tf.arg0(),
        tf.arg1(),
        tf.arg2(),
        tf.arg3(),
        tf.arg4(),
        tf.arg5()
    );
    let ret: isize = match syscall_num {
        // sys_exit 的语义是用户程序希望退出执行，它有1个参数，表示退出状态。
        SYS_EXIT => {
            println!("[SYS_EXIT]");
            // 将退出状态交给 axtask 并退出当前用户
            axtask::exit(tf.arg0() as _)
            // 注意，exit 是一个特殊的系统调用，它没有返回值。axtask::exit 会【中止当前执行流并销毁】，甚至当前这个函数 handle_syscall 都不会执行完
        }
        // 相对而言，write 是一个相对正常的系统调用。
        // 它完成系统调用之后将结果返回给用户
        SYS_WRITE => {
            println!("[SYS_WRITE]");
            // sys_write 有三个参数，分别代表文件标识符、输出数组指针、输出数组长度
            // 特别地，在程序启动时，文件表示符为 0 表示标准读入，即从键盘读入。
            // 文件标识符为 1 表示标准输出，为 2 表示标准错误输出，即使用 print 即可，目前不必区分。
            if tf.arg0() != 1 && tf.arg0() != 2 {
                // 当前实验不支持文件读写，所以这个内核不支持其他的文件输出。
                // 所以此处返回一个错误代码。
                // “错误代码”没有复杂的结构，它只是一个(有符号整数中的)负数。
                // 反之，如果系统调用执行成功，则应该返回0或者正数。
                // 错误类型定义在系统调用规范中，参见 ERRORS 部分，如 https://man7.org/linux/man-pages/man2/write.2.html
                // 执行成功时的返回值也记录在同页面中，在 RETURN VALUE 的部分。
                -LinuxError::EINVAL.code() as _
            } else {
                // 你可能已经注意到，我们的系统调用实现并没有覆盖规范中提到的所有情况，
                // 这对于小型教学内核的常态。
                // 我们通常只能支持它们被调用时的常见参数，而大量极少被调用的参数就只能遇到问题时再修补了。

                // 这里需要注意，用户传来的参数只是寄存器里的 64 位整数，
                // Rust 的编译器无法得知这些数值实际的类型和含义，也无法保证它们的安全。
                // 所以此处必须套用 unsafe 块，才能强行转换出一个 slice 类型用作数组。
                let user_data =
                    unsafe { core::slice::from_raw_parts(tf.arg1() as *const u8, tf.arg2()) };
                // 将它们转换成 utf8 字符串的过程也是 unsafe 的，因为数组里不一定所有的字符都是可打印的。
                println!("{}", unsafe { core::str::from_utf8_unchecked(user_data) });
                // 现在我们完成了用户的请求——将传入的字符串打印到标准输出（文件标识符为1）中
                // 只需要按照系统调用规范中的描述，返回成功输出的串长即可
                user_data.len() as isize

                // 上面的简化实现其实没有考虑【用户提供的数组指针非法】或者输出长度过长等情况。
                //
                // 下面提供另一种完成 sys_write 调用的方法。
                // 它使用当前进程 uspace 中提供的函数，在页表上先检查用户传入的地址是否存在映射，然后再依次翻译每个页面上的字符串。
                // 如果不存在映射，则会返回 user_data 为 None，从而可以被检查出来。
                // 这种做法更安全，但也更慢
                /*
                let current = axtask::current();
                let uspace = current.task_ext().aspace.lock();
                let user_data = uspace.translated_byte_buffer(tf.arg1().into(), tf.arg2());

                if let Some(data) = user_data {
                    let mut totoal_len = 0;
                    for str in data {
                        total_len += str.len();
                        println!("{}", unsafe { core::str::from_utf8_unchecked(str) })
                    }
                    total_len as isize
                } else {
                    -LinuxError::EFAULT.code() as _
                }
                */
            }
            // 接下来请回到 main.rs 中完成实验
        }
        _ => {
            ax_println!("Unimplemented syscall: {}", syscall_num);
            -LinuxError::ENOSYS.code() as _
        }
    };
    ret
}
