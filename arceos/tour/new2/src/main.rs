#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

mod buffer;
mod magic;
mod trap;

//make run A=tour/new2
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // 除了直接调用下层接口，内核还可以：
    // 2. 通过特权寄存器直接和硬件交互

    // TIPS：在本次实验中，./magic/ 下的代码文件包含一些硬件细节，不要求掌握。
    // 除此之外的代码文件是需要阅读学习的。

    // 如果说调用下层接口还只是函数调用的逻辑：给接口输入参数，之后拿到返回值。
    // 那么内核和硬件的直接交互就是魔法了。
    // 例如硬件可以施展以下几种常用魔法：

    // 2.1 异常中断
    // 当发生访问空指针之类的“异常”时，我们可以借由硬件自动跳转到某个没有任何人调用的函数，并记录相关信息。

    // 请前往 trap.rs 阅读 set_trap 函数的实现,
    // 简而言之，这个函数修改了硬件寄存器 stvec，使得在传入的函数执行过程中，
    // 异常会被 trap::example_trap_handler 函数捕捉。
    trap::set_trap();

    // 下面，我们构造一个指向地址 1 的指针
    let p: *const u64 = 1 as *const u64;
    // 然后尝试读取这个地址的值。
    // 这里的 read_volatile 是为了保证该行代码触发。否则，编译器发现 val 变量没有被实际使用，会优化掉这一行代码。
    let _val = unsafe { core::ptr::read_volatile(p) };

    // 如果在用户态运行上述代码，本程序将会报错退出。
    // 但当我们在内核使用魔法后，它在 example_trap_handler 中转了一圈后又重新回到了此处
    println!("Welcome back");

    // 在 x86, arm 等其他架构上，也有类似的机制，但流程略有不同。
    // 你【不需要】理解 ./magic 下的细节。
    // 本项目已经封装了类似本实验的内容，你可以直接使用在 axhal/trap.rs 中封装好的 IRQ, PAGE_FAULT, SYSCALL

    // EXERCISE 1：
    // 1. 将上面操作 stvec 的两行代码注释掉，重新运行，观察执行后的现象。
    // 2. 【保持上一题的状态】，使用 axhal/trap.rs 中的 PAGE_FAULT 宏编写一个函数，打印一行输出（可参考example_trap_handler）
    // TIPS：可以在项目中搜索 PAGE_FAULT 宏如何使用
    // TIPS：在第二题中，你【无法】回到本函数中打印 Welcome back，只能额外打印输出后退出。这是正常的。

    // EXERCISE 1 END

    // 这些功能是由硬件完成的，与 Rust 的编译器无关。
    // 因此，它不会遵循编译器负责的规范和逻辑：堆栈、函数的调用与返回等。
    // 相对地，编译器通常也不参与这些功能，也无法检查其中的错误。
    //
    // 所以内核程序员必须手动地、小心地使用和包装这些功能，完成衔接。
    // 如 ./magic/trap.S 中，我们手动维护了一个特殊的“函数调用和返回”，给编译器营造一个世界上没有魔法的假象。
    // 另一方面，在上面的实验代码中，我们又通过 read_volatile 告诉编译器，“这个值虽然没人读写，但仍然可能被改变过”，让编译器实际去内存中读取这个值，而不是优化掉这一行代码。
    // 这些功能的衔接只能由内核自己保证安全和正确性，Rust 的安全特性无法提供帮助。
    //
    // 后续实际编程中， axhal 已经写好了大部分接口，应对各种不同的硬件架构，只需调用即可。

    // 2.2 虚拟地址
    // 硬件上存在一种“虚拟地址”机制，可以让相同地址的指针访问到不同数据，让不同地址的指针访问到相同的数据，且可以动态调整。

    // 请到 buffer.rs 了解类型的细节
    use buffer::Buffer;
    // 创建两个 buffer，你可以把它们视为普通的向量或数组
    let mut page_a: Buffer = Buffer::new();
    let mut page_b: Buffer = Buffer::new();
    // 它们和数组的唯一区别是以 4096B 对齐
    let page_a_addr = page_a.data.as_ptr() as usize;
    let page_b_addr = page_b.data.as_ptr() as usize;
    // 4096 是 16 的 3 次方，所以以16进制表示时结尾会有三个0
    println!("page_a addr: {:#x}", page_a_addr);
    println!("page_b addr: {:#x}", page_b_addr);
    println!("{:#x}. {:#x}", page_a.paddr, page_b.paddr);
    // 它们可以正常读写数据
    let str_a = "Hello from A.";
    page_a.set_data(str_a);
    // [rust] 注意，裸字符串(&str)类型的 str_a 不需要任何支持， 但 str_b 其实是 String 类型的，需要 std::String
    // 在第一个实验中，我们已经说明在内核中没有 std，所以此处实际上是 axstd::String。
    // 而本文件开头的 extern axstd as std 骗过了编译器，使得此处的 str_b 不需要手动声明类型，而是自动适用 axstd::String 类型
    //
    // 注意，这种替代也经常骗过编辑器和检查。如声明如下语句：
    // use std;
    // 然后用 rust-analyzer(或其他 IDE 功能) 查看类型定义，会跳转到 Rust 标准库的代码中，但实际运行时内核中并不存在那些代码。
    // 这是因为代码检查功能（如 rust-analyzer）的编译选项与我们实际使用 make run 时调用的编译选项不一致。
    // 实验文件开头的 #[cfg(feature = "axstd")] 显示为暗灰色也是这个原因。这个问题其实可以修复。
    // 以 VSCode + rust-analyzer 为例，先 ctrl+, 调出配置界面，然后搜索 rust-analyzer + config，
    // 即可打开配置文件 config.json。在 json 中插入 "rust-analyzer.cargo.features": ["axstd"], 即可让 rust-analyzer 在编译时添加这一feature。
    // 如果你发现其他显示为暗灰色的实验代码实际上确实被运行了，也可以如上添加对应的 feature。
    // 特别地，对于依赖于架构的代码，可以添加 "rust-analyzer.cargo.target" : "riscv64gc-unknown-none-elf",
    // 即可指定运行架构。
    // 虽然本课程的目标是教授架构无关的内核编程，但前几个实验目前还是基于 riscv64 架构编写。
    //
    // 总之，当在本实验内核中见到 std:: 的代码时，它一定来自“假标准库” axstd。

    let str_b = "Hello from B. ".repeat(10);
    page_b.set_data(str_b.as_str());
    println!("page_a data: {:#}", page_a.get_data());
    println!("page_b data: {:#}", page_b.get_data());

    // 这次的魔法需要 map 和 check_paddr 两个函数
    use magic::{check_paddr, map};
    println!("\nmap a->b");
    // 在 buffer.rs 中，我们已经介绍过，软件程序只能看到“虚拟地址”，硬件实际访问的是另一个“物理地址”
    // 通过下面的 map 函数，我们将 page_a.data 的虚拟地址映射到 page_b 的物理地址
    map(page_a.data.as_ptr() as usize, page_b.paddr);
    // 映射之后，通过 page_a 的地址可以访问到 page_b 的数据
    println!("page_a data: {:#}", page_a.get_data());
    assert!(page_a.get_data() == str_b);
    // 但如果直接取裸指针查看，就会发现 page_a.data 的地址实际上没有改变，
    assert_eq!(page_a.data.as_ptr() as usize, page_a_addr);
    // page_a.data 的地址也没有改变。这是因为通过我们只是在通过软件程序去查看它们的地址。
    assert_eq!(page_b.data.as_ptr() as usize, page_b_addr);
    // 只有通过另一个魔法函数 check_paddr，才能看到两者的物理地址此时是一致的。
    assert_eq!(check_paddr(page_a_addr), check_paddr(page_b_addr));

    println!("\nmap b->a");

    // 但此时， page_a 中原来的数据并没有消失，只是不再被 page_a 映射
    //
    // EXERCISE 2：
    // 1. 尝试不做修改直接运行代码，此时下面的断言(assert_eq)会报错退出。这个报错是否会经过上面修改的example_trap_handler？如果没有，它又是在哪处理和输出的？
    // 2. 请参照上面的代码，使得 page_a 和 page_b 均映射到 page_a 的原数据，使得代码可以通过 EXERCISE 2 之后的断言。

    // EXERCISE 2 END

    assert_eq!(page_a.get_data(), str_a);
    assert_eq!(page_b.get_data(), str_a);
    assert_eq!(check_paddr(page_a_addr), page_a.paddr);
    assert_eq!(check_paddr(page_b_addr), page_a.paddr);
    println!("EXERCISE 2-2 passed");

    // EXERCISE 3:
    // 内核不一定要粘合硬件机制与编译器，也可以主动绕过编译器。
    // 例如，利用上述的地址映射，可以完全绕开Rust的所有权机制。
    // 你需要：完成下面调用的 write_to_static_var 函数的实现， 使得代码可以通过 EXERCISE 3 之后的断言。
    // 其中，不允许使用：unsafe 关键字、函数参数外的变量、修改过的函数声明、无理由的常量(如手抄一个page_b的地址)
    // 可以使用：write_to_static_var() 定义前的声明的函数和类型

    let str_c = "Goodbye from C.";
    write_to_static_var(&page_a, str_c);

    // EXERCISE 3 END

    assert_eq!(page_a.get_data(), str_c);
    assert_eq!(check_paddr(page_a_addr), page_a.paddr);
    println!("EXERCISE 2-3 passed");

    axhal::misc::terminate();
}

#[allow(unused)]
use buffer::Buffer;
#[allow(unused)]
use magic::{check_paddr, map};
fn write_to_static_var(_buf: &Buffer, _new_data: &str) {
    unimplemented!()
}
