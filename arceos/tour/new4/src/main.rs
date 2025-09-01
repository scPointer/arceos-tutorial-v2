#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]
#![feature(get_mut_unchecked)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

//make run A=tour/new4
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // 4. 内核启动与调试

    // 启动运行内核程序远远不只是“不能用标准库”，还包含许多我们没有介绍过的流程。
    // 在许多教程，如 rCore-Tutorial 中，内核启动才是教学起点第一章的内容。
    // 但由于 Arceos 的启动流程比较完善，包含大量新的功能和概念，所以为避免大家走马观花，在本实验我们才正式介绍 Arceos 的内核启动流程。

    // 4.1 引子：总体启动过程

    // 从第一节实验开始，我们有意忽略了一个事实：内核程序并不是从 main 开始执行的。
    // 有许多迹象其实暗示了这一点：
    // a. 在第二节实验的 EXERCISE 1 的第一题中，即使不设置 stvec，非法访问依然触发了异常报错，只是位置不同。
    // b. 在第二节实验的虚拟地址部分的输出中，每个 buffer 的虚拟地址和物理地址本就不同。
    // 事实上，map() 并不是在空白中创建了一对映射，而只是在 Arceos 内核已经建立好的映射中做细微修改。
    // 运行下面的代码可得到关于当前地址映射的信息。它们都是 Arceos 在当前这个 main 函数之前完成的。

    use axhal::mem::memory_regions;
    for region in memory_regions() {
        println!("{:#?}", region);
    }

    // c. 在每节实验的开头第二行都有 #![cfg_attr(feature = "axstd", no_main)]。
    // 根据第三节实验的 Rust 属性解析，这表示只要 feature 包含 axstd，那么就为本文件添加 no_main 属性。此处本不应该有一个 main 函数存在。

    // 事实上，启动的过程可以大致描述为：

    // 1. 下层驱动/硬件启动，并负责将内核的代码放置到一个硬件指定的地址。这个地址写在 arceos/platforms/ 的各个文件中，即 kernel-base-paddr 变量。
    // 2. 内核为自己准备栈、堆等必要结构的初始化，包括上面 a,b 提到的异常中断与地址映射等硬件功能。
    // 3. 跳转到本文件——一个名字恰好为main但其实被编译器视作普通函数的地方——执行。
    // 4. 在下一章实验中，这个main()函数也是初始化的一部分。我们将在这里为用户程序准备环境，然后跳转到用户态执行。

    // 本节实验我们主要讲上述的前二个部分，即从内核启动到 main 的过程中，发生了什么。

    // 4.2 内核启动之前

    // 先从内核启动后的输出讲起。请运行之前教程第一节实验的代码，并观察输出：
    // make run A=tour/new1

    // 在 Rust 编译过程之后，先有一行 rust-objcopy 开头的输出，生成一个 .bin 的二进制文件，包含内核所有代码。
    // 随后有一行 qemu-system-riscv 开头的命令输出，启动虚拟机，在你的电脑的用户态模拟出一台 RISC-V 架构的机器。
    // 命令中的参数即是这台虚拟机器的配置。

    // 在本教程中，这台机器包含机器态(M)/内核态(S)/用户态(U)三种【特权级】，机器态权限最高，用户态权限最低。
    // 开机时处在机器态(M)，运行 qemu 自带的 OpenSBI，也就是输出中的 OpenSBI Logo 及一系列参数。
    // OpenSBI 在完成自己的初始化后，还会把内核代码加载到 0x8020_0000 这个地址，并跳转运行。如第一节实验所示，从此之后，内核运行时也可调用 OpenSBI 来完成读取字符/写入字符/关机等功能。

    // EXERCISE 1:

    // 1. 内核启动地址、qemu内存大小等这些配置信息和参数不仅被启动脚本使用，还可以在内核运行中读取。这是由 axconfig 模块提供的功能。请分析代码，解释 axconfig 如何把文本配置文件变成运行时可调用的参数。
    // TIPS：[rust] Rust在编译时就会运行库中的 build.rs。
    // 2. 修改下述代码，然后运行本文件（make run A=tour/new4），想办法通过 axconfig 打印出内核启动地址、qemu启动时的内存大小、启动核数。

    //use axconfig::{...};
    //println!("{:x} {:x} {}", ?, ?, ?);

    // EXERCISE 1 END

    // 4.3 内核启动之后

    // 内核刚启动时，除了调用下层 OpenSBI，其他所有初始化操作都需要自己完成。

    // 内核启动的第一条语句在地址 0x8020_0000 处，它在 modules/axhal/src/platform/qemu_virt_riscv/boot.rs 文件的 _start 函数中。下面请【打开该文件，对照阅读接下来的教程】。

    // 4.3.1 先回答几个你可能会好奇的“为什么”：

    // Q：为什么内核入口是 0x8020_0000 这个地址？
    // A：这其实不是配置中可以任意修改的参数，它是由 OpenSBI 规定的。它需要占用 0x8000_0000 到 0x8020_0000 这段地址。
    // Q：为什么 0x8020_0000 处是这个函数？
    // A：因为在函数开头指定了 #[link_section = ".text.boot"]。这个 .text.boot 段的名字并不特殊，只是因为在 axhal/linker.lds.S 文件中把 .text 放在开头，而 .text.boot 又是这一段的开头。这个文件被交给编译器，以确定代码编译后的排布。
    // Q：为什么这个函数的名字是 _start？
    // A：因为这个符号是 C 指定的程序入口点。在本节实验开头提到，文件开头的 #![cfg_attr(feature = "axstd", no_main)] 也指定了本文件其实没有入口 main 函数，而真正的入口是 _start。

    // 4.3.2 接下来正式过一遍这个函数（_start）的流程：

    // 首先保存 OpenSBI 传递给内核的两个参数：当前核心编号和设备树(DTB)指针。
    //      mv      s0, a0                  // save hartid
    //      mv      s1, a1                  // save DTB pointer

    // 然后设置栈指针。Rust 等高级语言在编译时均假设栈存在，必须先有栈才可以进入 Rust 代码申请局部变量乃至调用函数。
    // [rust] 在 _start 函数被”调用“时并没有设置栈。之所以可以这样，是因为函数前的[naked]标签告诉Rust编译器不要为这个函数生成压栈退栈的代码。
    //      la      sp, {boot_stack}
    //      li      t0, {boot_stack_size}
    //      add     sp, sp, t0              // setup boot stack

    // 初始化页表，即第二节实验提到的虚拟地址映射。
    // 你可以在同文件中找到下面两个函数的代码，但不要求理解。
    // 总之，这里把虚拟地址[0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000]和[0x8000_0000..0xc000_0000]都映射到物理地址[0x8000_0000..0xc000_0000]。
    // 这是利用 RISC-V 的“大页”机制做的粗粒度映射，还需要后续操作才能变成本文件开头输出的映射。
    //      call    {init_boot_page_table}
    //      call    {init_mmu}              // setup boot page table and enabel MMU

    // 之后栈指针也需要经过第二节实验 buffer.rs 中的虚实转换，改成虚拟地址。
    //      li      s2, {phys_virt_offset}  // fix up virtual high address
    //      add     sp, sp, s2

    // 最后调用 entry 符号指定的函数。
    // 在同文件中可以找到它实际是 entry = sym super::rust_entry
    // 也就是 modules/axhal/src/platform/qemu_virt_riscv/mod.rs 中的 rust_entry
    //      mv      a0, s0
    //      mv      a1, s1
    //      la      a2, {entry}
    //      add     a2, a2, s2
    //      jalr    a2                      // call rust_entry(hartid, dtb)

    // 4.3.3 现在我们来到 modules/axhal/src/platform/qemu_virt_riscv/mod.rs 的 rust_entry 函数，请【打开该文件，对照阅读接下来的教程】。它的流程相对简单：
    // 先清空 BSS 段，这里保存着所有未初始化的全局变量。
    //crate::mem::clear_bss();

    // 当前 CPU 核心的初始化，这在多核时有用。但本教程自始至终是单核启动，跳过介绍。
    //crate::cpu::init_primary(cpu_id);

    // 设置异常中断处理函数的入口地址。这也就是第二节实验中异常中断的设置。
    // 如果第二节实验没有修改 stvec，那么会默认跳转到此处初始化时输入的函数。
    //crate::arch::set_trap_vector_base(trap_vector_base as usize);

    // 初始化实时时钟，这仅在开启 rtc 特性时有用，跳过介绍。
    //self::time::init_early();

    // 下一步，跳转到 axruntime 中的 rust_main 函数执行
    //rust_main(cpu_id, dtb);

    // 4.3.4 现在我们终于脱离了 axhal，来到 modules/axruntime/src/lib.rs 中的 rust_main 函数，开始全平台的通用初始化。请【打开该文件，对照阅读接下来的教程】。这个函数比较长，就不再贴代码了，只介绍几个重要的步骤：

    // a. 打印 ArceOS 的 Logo 和一些配置信息。
    // [rust] 与 axconfig 那样由 Arceos 自己写规则管理的配置不同，此处的配置信息使用 Rust 提供的 option_env! 宏，是 Rust 管理的全局环境变量。
    // 例如你可以去掉下面代码的注释
    // println!("{}", option_env!("ABC").unwrap_or("Not found"));
    // 然后运行 make run A=tour/new4
    // 再试试运行 make run A=tour/new4 ABC=123

    // b. 初始化 axlog 库提供的日志机制，此后就可以使用 info/warn/error 等宏了。
    // c. 初始化全局内存分配器，也就是堆。此后可以动态申请 Vec/Box 等结构的内存。
    // d. 初始化页表，这一步重新生成页表，更精细地调整地址映射。例如堆上的数据可读可写但不能执行，再例如代码段可读可执行但不能被覆写。这也就有了本文件开头的地址映射信息输出。
    // e. 初始化架构特定的设备。在 RISC-V 中，则是启动中断和计时器功能，但暂时未设置。
    // f. 初始化调度器。此后可以生成新的执行流，交给调度器调配。
    // g. 初始化文件系统、网络、显示、TLS、多核启动等功能，实验目前暂不涉及。
    // h. 设置时钟中断，保证其每隔固定时间触发。
    // i. 调用名为 main 的函数，也就是本注释所在函数
    // j. nain 函数结束后，等待所有执行流都结束后，关机。

    // EXERCISE 2:
    // 1. 在 boot.rs 中，启动栈属于 .bss.stack 段，后续的函数调用都发生在该栈上。为什么 rust_entry 函数清空 .bss 段时没有把自己锁在的栈也清空？
    // TIPS：查看 clear_bss 函数的实现以及上文提到的 linker.lds.S 文件。
    // 2. 回顾启动流程，分别回答：内核最早可在什么时候使用下述功能？
    // a. 实验一提到的下层 OpenSBI 的接口
    // b. 实验一提到的 getchar/putchar/terminate 这些 axhal 中定义的 Rust 函数
    // c. 实验一提到的 axstd 中的 print! 宏
    // d. 实验二提到的异常入口(trap)机制
    // e. 实验二提到的虚拟地址映射
    // f. 实验三提到的panic
    // g. 实验三提到的动态内存分配
    // h. 实验三提到的使用 thread::spawn 创建新执行流
    // i. 实验三提到的时钟中断

    // EXERCISE 2 END

    // 4.4 内核调试

    // 本节实验需要阅读理解多个 Arceos 内部模块的代码，之后的实验只会更加复杂。
    // 为帮助大家完成后续实验，本节额外介绍一些在内核中常用的调试方法。

    // 4.4.1 搜索——“这行报错输出是哪来的？”

    // 本节的练习一的实验要求同学在 Arceos 中搜索特定字符串或语句。
    // 此时使用 grep 命令或者 IDE 提供的搜索功能都是比较常规的做法。
    // 更多用法参考： https://scpointer.github.io/rcore2oscomp/docs/lab2/pos.html

    // 4.4.2 LOG输出——“这个变量的值为什么不对？”

    // 此时最简单的方法是在出错代码之前插入调试语句，然后重新运行。
    // 首先，你当然可以使用第一节实验提到的 println! 宏。
    println!("println from axstd");
    // 但许多内核模块在 axstd 下层，被 axstd 所依赖，无法直接打印。
    // 此时可以在 Cargo.toml 中添加 axlog 或者 log 模块的依赖，然后使用 axlog 提供的日志宏。
    // 包括 trace!/debug!/info!/warn!/error!，等级依次提高。

    // 此外，并不是每条 LOG 语句都会被输出。默认只有等于高于或等于 warn 的宏会被输出。
    // 你可以在使用类似 make run A=tour/new4 LOG=info 的命令来调整输出等级。

    // 使用LOG输出时有一些注意事项：
    // a. 不要依赖 Arceos 原有的输出语句，调试时可以【大胆插入新的】！
    // b. 一些函数执行频率过高，可能导致输出刷屏。此时先按ctrl+A，松开后再按X可以【强行终止 qemu 虚拟机】，然后再做考虑。
    // c. 添加输出语句后，如果出现第二节实验练习一的异常中断报错，说明可能【变量地址为空或非法】。此时可以尝试先输出变量地址，如 warn!("{:p}", &var);
    // d. 如果发现添加输出语句本身辉影响代码执行结果，则大概率是栈溢出等内存问题。【加了条输出结果居然变了】时优先检查栈大小、虚拟映射以及是否在 unsafe 中读写了野指针。

    // [rust] 输出16进制数、结构体内容等特殊格式可参考 https://doc.rust-lang.org/rust-by-example/hello/print.html

    // 4.4.3 反汇编——“程序异常崩溃了，但找不到在哪？”

    // 此时一般还是先建议用之前的方法，在内核各处插入任意的LOG输出，排查出错位置。
    // 实在找不到时可以先【编译通过一次】，然后在实验模块的目录下找到形如 xxx_riscv64-qemu-virt.elf 的文件。
    // 然后即可使用如下语句，将内核代码范汇编至文本文件（以 1.S 为例）
    // riscv64-linux-musl-objdump tour/new4/new4_riscv64-qemu-virt.elf -ld > 1.S

    // 下一步，我们观察异常的输出（第二节实验练习一），例如：
    // Unhandled Supervisor Page Fault @ 0xffffffc080200688, fault_vaddr=VA:0x1 (READ):
    // 其中开头标注了异常原因，以 0xffffffc08020... 开头的是出错的代码地址
    // 在反汇编的文本文件中找到该地址所在行即可。

    // 其他架构的异常输出、反汇编文件的格式也是类似的。
    // 这种调试方法需要同学比较熟悉对应架构的汇编语言。

    // 4.4.4 GDB调试——“能不能像普通用户程序那样，正常动态单步调试内核？”

    // Arceos 支持 gdb 调试功能，只需要将运行指令从 run 改为 debug 即可：
    // make debug A=tour/new4

    // 调试的指令和注意事项可参考： https://scpointer.github.io/rcore2oscomp/docs/lab2/gdb.html#%E7%B2%BE%E7%AE%80-gdb-%E5%91%BD%E4%BB%A4
    // 上面的教程是针对 rCore 的 GDB 调试写的，Arceos 与之有些区别：
    // a. Arceos 的 GDB 输出和代码运行输出在同一个窗口里，不需要开两个终端
    // b. Arceos 默认打一个 rust_entry() 函数的断点，并直接运行到此处，不需要从 0x8020_0000 开始打断点。
    // c. Arceos 中同一个执行流的用户态和内核态使用同一个页表，进入用户态时无需切换（后续实验讨论）。

    // EXERCISE 3:

    // 1. 内核启动地址 0x8020_0000 以及虚拟机内存大小 128M 是在哪个文件中定义的？在 qemu 启动前输出的“Runing on qemu...”又是在哪个文件输出的？
    // 2. 参考其他模块（如 axruntime），在本实验中添加适当语句引入 axlog，使得下面几条命令可正常编译运行
    //info!("log info");
    //warn!("log warn");
    //error!("log error");
    // 3. 反汇编本实验代码，分别写出 _start/rust_entry/rust_main/main 这几个函数的入口地址。然后分别写出它们是从哪一行开始被前一个函数跳转执行的。
    // 4. 本节实验的末尾没有调用第一节实验提到的 terminate() 函数。事实上，之前实验中的这条调用也是不需要的，去掉后仍可正常退出。请根据本节内容分析原因。

    // EXERCISE 3 END

    axhal::misc::terminate();
}
