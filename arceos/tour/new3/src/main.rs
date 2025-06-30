#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

use std::thread;
use axdriver::prelude::{DeviceType, BaseDriverOps, BlockDriverOps};

use std::os::arceos::modules::axhal::mem::memory_regions;
use axstd::vec::Vec;

mod magic;

const DISK_SIZE:    usize = 0x400_0000; // 64M
const BLOCK_SIZE:   usize = 0x200;      // 512-bytes in default

//make run A=tour/new3
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {

    // 3. 使用软件代码创建新的接口

    // 在前两个实验中，我们让内核直接转接下层的接口，也尝试通过硬件完成一些神奇的操作。
    // 但还有很多接口既不依赖下层，也几乎不需要硬件的魔法参与，而是靠内核自己用“科学”创造

    // 
    // 本章介绍线程、管道、fdtable

    // to be continued...


    



    // ex. 和语言与编译器相关的接口、no_std 
    // 你可能注意到，在第一个实验中，我们说在内核的程序无法使用任何你所熟悉的接口
    // 但在第二个实验中，我们堂而皇之地使用了一个 core::ptr::read_volatile 函数
    // 这里的 core 库是 std 库的一部分。
    // 
    // 更确切的情况是，在内核之上、Rust 标准库之下有一层称为 syscall 的接口中。
    // 大部分标准库函数依赖这层接口，则必须要有内核支持才可运行;
    // 另一些函数（如指针操作、字符串处理、编译器提示）则完全不需要 syscall，因此可以在内核中直接使用。
    // 类似本章实验，这些不需要 syscall 的代码也几乎不依赖硬件，是软件创造的接口。
    // 如果要使用“没有内核支持的环境”，则需要在 main.rs 开头标注 [no_std]，编译器会自动检查是否可以在内核运行。
    //
    // axstd 库本质是在 [no_std] 的环境中，构造一些和 Rust 标准库名字一样的接口，使得用户程序可以在内核运行。
    // 当然，此时所有下层接口完全依赖 axstd 中的实现，与真正的标准库没有关系。
    axhal::misc::terminate();
}
