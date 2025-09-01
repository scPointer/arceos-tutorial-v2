#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

//make run A=tour/new1
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // 本教程作为开源操作系统训练营教程，安排在 rustlings 课程之后，实践实习之前，与操作系统课程同期开展。
    // 本教程相当于旧教程中 rCore-Tutorial 和 Arceos-Tutorial 两部分，为帮助不熟悉操作系统内核的同学快速上手 Arceos 而设计。

    // 实验说明
    // 1. 教程前5节的实验为体验与报告形式，包含少量代码作业；后续实验为代码评测形式，包含需要评分的代码模块。以 `// EXERCISE` 开头的注释是需要完成的实验内容。
    // 2. 教程中标注[rust]的段落为 Rust 语言相关知识，已熟悉 Rust 的同学可跳过。
    // 3. 每节教程可按 main.rs 中 main() 函数的注释顺序阅读，除特殊标注外，不需要完全理解其他文件的内容。
    // 4. 教程的目的是“快速上手”，有时仅给出各个功能接口的简单介绍。如需详细了解对应内核功能的内部实现，请查阅标注的扩展阅读或询问老师。

    // ---------- 下为正文部分 ----------

    // 把程序放在内核中运行，能多做什么？不能做什么？

    // 原有的标准库，如 cstdio 和 Rust 的标准库都是运行在内核之上的，它们的接口都无法在内核中使用
    // 所以，程序原本调用的任何功能——比如打印字符串、打开文件——都不存在
    // 内核需要用更下层的接口去“实现”这些功能，并提供给更上层的用户

    // 在内核中有哪些可以使用的功能/接口？

    // 1. 调用更下层的驱动和接口
    // 虽然没有任何熟悉的接口，但在内核中我们并非孤身一人。
    // 我们可以直接调用硬件驱动和下层接口来完成操作。例如，在RISC-V 架构上，有 OpenSBI 定义的接口。
    // 扩展阅读：https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/src/ext-legacy.adoc
    // 在本课程中，我们【不会】介绍硬件驱动与下层接口的实现细节，而是直接使用它们提供的功能。

    // 很不幸，这样现成的接口非常有限。我们只会使用 读取字符/写入字符/关机 三个功能
    #[allow(unused)]
    use axhal::console::{getchar, putchar};
    use axhal::misc::terminate;
    // TIPS：为了教学需要，部分的 'use' 语句会像这样直接写在函数中。但更规范的 Rust 代码应在文件开头引入这些外部符号。

    let hello_world: &str = "Hello world\n";
    for c in hello_world.bytes() {
        putchar(c);
    }
    // 内核不会在所谓运行结束时“自动退出”，必须要明确通过指令退出。
    // 尝试去掉下面这条注释，当前程序会立即结束

    // terminate();

    // EXERCISE 1：
    // 1. 在项目中找到 axhal 中的 getchar 的声明和实现。
    // 这个函数实际调用了哪个实现？是如何选择的？（不需要理解更下层的逻辑）
    // 2. 使用 getchar 函数在此处实现一个“在此处暂停，按任意键继续”的效果

    // EXERCISE 1 END

    // 在上面的小实验中，你会看到这几个简单接口背后的实现可能非常复杂
    // 这就是 axhal 存在的意义。HAL 是硬件无关层的意思，将不同架构的功能统一起来，供上层(内核)使用
    // 我们后面还会再介绍这个模块

    // 在浏览其他代码时，你可能发现打印字符串不一定要用 putchar，而是随时都可以：

    print!("Again, {}", hello_world);

    // 这是因为本文件开头引入了一个叫 axstd 的模块
    // #[macro_use]
    // #[cfg(feature = "axstd")]
    // extern crate axstd as std;
    //
    // 其中 #[macro_use] 表示使用这个引入模块内部提供的宏。
    // axstd 包装整合了包括 getchar / putchar 在内的接口，在内核中“模拟”了一部分的 Rust 标准库。
    // axstd 并不是真正的标准库，我们只是通过 extern crate axstd as std 重命名它，让本文件可以“在内核与用户态写一样的代码”。
    // 实际上的内部实现是完全不同的。在用户态实验中会详细讨论。
    //
    // 相较而言，axhal 更下层的模块，本实验中不需要关心内部的实现；
    // 而 axstd 是上层模块，我们既调用它的接口，也需要学习理解它的实现。
    // 我们后面还会再介绍这个模块

    // EXERCISE 2:
    // 1. axstd 模块是如何一层层调用到 putchar 的？简要画出函数调用关系图。

    // EXERCISE 2 END

    terminate();
}
