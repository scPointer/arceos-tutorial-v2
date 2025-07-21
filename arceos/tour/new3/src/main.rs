#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]
#![feature(get_mut_unchecked)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

//make run A=tour/new3
#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // 3. 使用软件代码创建新的接口

    // 在前两节实验中，我们让内核直接转接下层的接口，也尝试通过硬件完成一些神奇的操作。
    // 但还有很多接口既不依赖下层，也几乎不需要硬件的魔法参与，而是靠内核自己用“科学”创造。

    // 3.1 Rust 属性

    // 在第二节中，我们使用 assert_eq! 宏来判断两个值是否相等，也有类似的 assert! 和 assert_ne! 等等宏实现类似的功能。
    // 它们的本质都是在特定情况下调用 panic! 宏。你可以取消下面这行代码的注释，然后尝试重新运行本实验，查看结果
    //panic!("exit now");
    //
    // 如果已做过第二节的实验，你会发现这条语句会调用到 modules/axruntime/src/lang_items.rs 中的函数。
    // 这是因为这个函数的开头打上了 #[panic_handler] 标记，所以会被 panic! 宏自动调用。
    //
    // 这与第二节中访问非法指针的实验不同。
    // 访问非法指针时，整个CPU确实地在执行指令的过程中暂停了正常运转，触发特殊流程，记录异常的类型和位置等信息后，跳转到异常处理函数；
    // 但使用 assert! 或 panic! 时，我们只是在软件层面检查变量的值，并完成函数调用。
    // 你看不到从宏到 #[panic_handler] 的调用过程是由 Rust 的编译器补充的，和硬件无关。

    // 类似地，动态内存分配也是由编译器调用的。
    // 在 modules/axalloc/src/lib.rs 中可以看到下面的代码
    //#[cfg_attr(all(target_os = "none", not(test)), global_allocator)]
    //static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator::new();
    //
    // [TODO] 了解什么是堆、什么是栈
    // 当 Vec 这类变量被分配在堆上的时候，需要调用类似 C 中的 malloc 或者 C++ 中的 new() 来获取一段内存。
    // 而在 Rust 中，实际被调用的是有 #[global_allocator] 标记的变量，以及它锁实现的 trait GlobalAlloc。
    // 如果删去本实验的 Cargo.toml 文件中的 axstd 的 "alloc" 这一属性，则无法再使用动态内存分配功能。

    // [rust] 属性的阅读规则
    // 以本文件中的语句为例，简要介绍一下 Rust 属性的规则：
    // 1. 井号带感叹号的属性(#!)对当前文件生效，不带感叹号的属性(#)只对下一条语句或块生效。
    // 2. #[cfg(feature = "axstd")] 表示当 feature 中包含 axstd （引入该模块）时，执行下一条语句。
    // 3. #[cfg_attr(feature = "axstd", no_mangle)] 中 cfg_attr 表示前一个参数成立时，生成后一个参数。这里表示当前 main 函数带 #[no_mangle] 标签，这在第二节的 trap.rs 中有介绍。
    // 4. #[cfg_attr(all(target_os = "none", not(test)), global_allocator)] 中的 all 表示逻辑与，not 表示逻辑非。类似的还有 any。

    // [rust] no_std 属性
    // 在目前每节实验的开头，都有一行 no_std 属性，表示当前环境中没有 Rust 标准库。
    // 在内核之上、Rust 标准库之下有一层称为 syscall 的接口，标准库通过它来使用内核功能。
    // 现在我们就是内核，无法直接调用标准库，但并不是完全不能使用标准库中的函数。它们可以分为三类：
    // 1. 底层调用 syscall 的函数无论如何无法运行，只能内核自己来写，我们会在后续的实验中介绍并实现它们;
    // 2. 不需要依赖 syscall 的函数（如 axstd 中调用的 alloc 和第二节实验的 core）则可以直接调用；
    // 3. 还有函数通常需要依赖内核实现（本节的 #[panic_handler] 和 #[global_allocator]），但又不可或缺，只好在 no_std 环境下留出接口，让内核实现之后通过 Rust 属性对接。

    // 3.2 多执行流与主动切换

    // 你可能听说过进程、线程这些概念，指在通常一行行顺序执行的程序中，分出一部分可以在同一时间并行执行的程序。
    // 进程和线程的概念及区别会在后续实验讲解，在本实验我们先暂时将并行执行中的每个程序称作一个【执行流】。
    // 一个简单的想法是，用多个CPU分别各自运行一个执行流。但这种并行并不局限于CPU的个数，Linux可以同时运行成千上万个不同的进程。
    // 特别地，在本实验中，我们仅有一个CPU，但依旧可以模拟出同时并行运行的多个不同执行流。
    //
    // 与第二节不同，在一个CPU上没有架构天生用于支持多执行流。（并行指令和超线程等概念和本节内容无关，此处的CPU指从硬件上层的内核角度看到的逻辑核心）
    // 无论是通用寄存器，还是执行第二节魔法的各自特权寄存器，都只有一组。
    // 这个“在一个顺序执行指令的CPU上运行多个不同执行流”的过程几乎完全是内核自己完成的。
    // 下面我们先演示这一过程。

    // 首先，我们定义一个用 Arc 包裹的整数变量。
    // Arc 是 Rust 的智能指针，可以在多执行流环境中保护内部变量，它自身也可以安全地复制、转移。
    // 它内置一个引用计数功能，会在无人引用时自动销毁内部变量。
    use std::sync::Arc;
    let var = Arc::new(0usize);
    // [rust] 此处声明了一个 lambda 函数，或者用 Rust 的语言说，是一个闭包。
    // 它首先可以视为一个函数，在两个竖线之间写参数，在之后写函数体。
    // 如 fn add(x:u32) -> u32 { x + 1 } 相当于 let add = |x| { x + 1 };
    // 我们稍后会讨论闭包和通常函数的区别。
    let thread_fn = |mut var: Arc<usize>, name: &str| {
        // [rust] Rust中用单下划线表示“无名变量”，后续不会也无法被使用，只是一个占位符。也可以用在生命周期等场景。
        // 也可以使用以下划线开头的变量名或参数名，“暗示”编译器它后续不会被使用。但这样的变量仍然可以正常使用，例如第二节实验中的 write_to_static_var 函数参数。
        for _ in 0..10 {
            // 此处我们获取了 var 内部保存整数的可变引用。
            // 因为我们没有使用锁或者原子变量，所以这个整数只能安全地被多执行力读取，不能安全写入。
            // 本实验为了演示，强行让多线程可以写入这个变量，这实际上是不安全的，因此使用了 unsafe
            let var_mut = unsafe { Arc::get_mut_unchecked(&mut var) };
            // 如果是多核运行，下面这行将有更大概率触发冲突，使 var_mut 的最终值变动。
            *var_mut = *var_mut + 1;
            println!("{name} {var_mut}");
            std::thread::yield_now();
        }
    };
    // 复制一个 var 指针，它的内部指向同一个整数。
    let var1 = var.clone();
    // 上面的 thread_fn 可以作为一个正常函数使用，我们可以直接调用它。此处【没有】执行流切换。
    thread_fn(var1, "t1");
    // 我们可以使用 axstd 提供的 spawn 函数创建一个新的执行流。它的使用方式和真正的 std 的 spawn 类似。
    use std::thread::spawn;
    let var2 = var.clone();
    // 这里我们创建了一个新的闭包作为 spawn 的参数。它的唯一操作是调用 thread_fn
    // 注意，它的前面多了关键词 move，而且没有参数。
    // [rust] 这个闭包明明调用了 var2，为什么没有写在参数里？
    // 这是因为 Rust 闭包可以自动捕获外部变量，实际上是将变量的引用作为参数。
    // 这种行为可以被其他参数修改：
    // 如果定义闭包时把变量定为可变，如 let add = || { x+=1; } 则可以修改捕获变量的值；
    // 如果如下述代码加上 move，则可以将变量所有权(而非只是引用)转移给闭包。
    let t2 = spawn(move || thread_fn(var2, "t2"));
    let var3 = var.clone();
    let t3 = spawn(move || thread_fn(var3, "t3"));

    // 内核中存在“任务队列”保存所有正在运行的执行流，
    // 而被扔进 spawn 的函数会【立即】进入队列，不需要额外的“启动”步骤。
    // 我们只需要通过 join 函数等待这两个执行流结束即可。
    t2.join().unwrap();
    // 这里的 join 的返回值是一个 Result，告诉我们执行流是否运行成功，以及对应的返回值。
    // [rust] Rust 要求必须处理 Result 类型的返回值，哪怕只是“主动忽略”它。
    // 所以这里使用了 unwrap() 表示如果返回值为 Ok(...) 则什么也不做；反之返回 Err(...)，则直接通过 panic! 退出
    t3.join().unwrap();

    // EXERCISE 1：
    // 依次尝试下面的操作，运行本实验，然后复原。描述你看到的现象(运行成功或看到的报错)，并回答对应问题。
    // 1/ 删除上述代码中的变量 var2 和 var3，直接将 var.clone() 写在参数里。解释报错的原因。
    // 2. 定义一个 let t = move || thread_fn(var2, "t2"); 然后尝试直接调用 spawn(t)。这个 t 是什么类型？和 thread_fn 有何不同？
    // 3. 删除上述代码中的 move ||，即直接调用 spawn(thread_fn(var2, "t2"))。解释 spawn 函数的类型声明中的各个类型和关键词。
    // 4. 将上述代码中的 var2 和 var3 都改名为 var1（无论定义处还是使用处）。如果运行报错，解释报错的原因；如果运行成功，解释为什么可以这样写。

    // EXERCISE 1 END

    // 上面的代码通过 spawn 让多个执行流并行运行，但实际上本实验中只有一个 CPU。
    // 所以真实的运行过程其实是，这唯一的 CPU 每次只会运行其中一个执行流中的一段代码。
    // CPU 是一个玩家，而每个执行流是一个存档。CPU 每次读取其中一个存档，
    // 运行直到发生某些事件后，保存当前存档然后读取下一个存档。
    //
    // 存档的“组成”是什么？换句话说，我们需要保存一个执行流的什么信息，才能在中途切走之后还能再切换回来，同时还不会出现bug？
    // 这样的存档被称作【上下文】。目前我们只需要知道它存在，暂时不需要了解它的构成。
    // 如果实在好奇，可以看看本实验架构的 ['arceos/modules/axhal/src/arch/riscv/context.rs'] 中的 ['TaskContext']

    // EXERCISE 2：
    // 运行EXERCISE 1之前的代码，输出一定是 t2 和 t3 严格一一交替输出，即使把内部循环次数从 10 改成 100 或更大也是如此。
    // 1. 是什么代码导致了这一现象？写出函数调用链条，从本实验中的函数开始，一直找到上面介绍的“存档读档”的代码为止。
    // 2. 去掉对应的代码，使得重新运行本实验后， t2 和 t3 不再交替输出。尝试解释此时的输出。
    // 3. 恢复前两题的修改，然后将 t2.join().unwrap(); 和 t3.join().unwrap(); 删除，换成 std::thread::yield_now()； 写出此时的输出。然后再多写一条 yield_now()，写出并解释此时的输出。

    // EXERCISE 2 END

    // 3.3 时钟中断导致的切换

    // 在上面的代码中，我们通过软件代码实现了多执行流并行运行。
    // 但这种方式需要每个执行流在自己的任务中主动插入 yield_now 这样的挂起代码，并且小心处理挂起和执行的顺序，才能正常运行。
    // 我们希望用一种方法，使得每个执行流无需手动切换，就可以在执行一段时间后自动让出CPU给其他执行流。
    // 这样不仅不需要插入切换代码，还能避免一个执行流长时间占据CPU，更公平。

    // 这需要借助一些硬件的力量。除了软件代码主动调用导致的上下文切换之外，其实也有硬件触发的切换。
    // 这个机制叫做“时钟中断”。简单来说，通过硬件设置，可以用第二节实验介绍的异常中断机制强行打断CPU，
    // 然后再从陷入函数(trap)中调用切换函数。
    // 这样的打断每隔一段时间发生，内核可以通过设置硬件寄存器来开关它，也可设置触发的时间间隔。

    // 用下面的命令重新运行本实验（注意末尾新增的参数）
    //make run A=tour/new3 APP_FEATURES=irq
    #[cfg(feature = "irq")]
    {
        // 代码是类似的，但有了中断之后，也可以利用 sleep 函数来挂起执行流。
        // sleep 只能保证当前执行流在睡眠时间内不会再执行，并不保证到时候立即执行。
        // 特别地，如果没有时钟中断，则 sleep 的行为是死循环等待时间到达，然后继续执行，并不会切换其他执行流。
        use std::thread::sleep;
        use std::time::Duration;
        let var = Arc::new(0usize);
        let thread_fn = |mut var: Arc<usize>, name: &str| {
            for _ in 0..10000 {
                let var_mut = unsafe { Arc::get_mut_unchecked(&mut var) };
                *var_mut = *var_mut + 1;
                println!("{name} {var_mut}");
                // 等待5ms
                sleep(Duration::from_millis(5));
            }
        };
        let var2 = var.clone();
        let t2 = spawn(move || thread_fn(var2, "irq-t1"));
        let var3 = var.clone();
        let t3 = spawn(move || thread_fn(var3, "irq-t2"));

        // 此处演示使用 sleep 来等待其他执行流。当然，也可以继续使用 join。
        sleep(Duration::from_secs(1));
        // 你可能会认为，调用 sleep 本质上也是一种“主动”的调度。所以我们也提供了另一种方式：
        //massive_work(10_000_000);
        // 你可以尝试删除或注释掉前面的sleep，然后使用上面这行 massive_work 来模拟死等循环计算。
        // 只要循环次数足够，仍然可以看到其他的执行流被切换出来并执行。
        // 这就是硬件的时钟中断完成的切换。
    }

    // EXERCISE 3:
    // 对于上面 #[cfg(feature = "irq")] 中的代码：
    // 1. 删除 thread_fn 内部的 sleep 语句，重新运行程序，观察并描述输出。
    // 2. 在上一题基础上，将 thread_fn 内的循环次数从 10 改到 100 或更大，观察并尝试解释输出。
    // 4. 在上一题基础上，继续增加循环次数，然后将 thread_fn 内的输出语句改为每循环若干次才输出一次，观察并尝试解释输出。

    // EXERCISE 3 END

    // 上面的实验说明，涉及下层的一些接口其实非常耗时。

    // 在本实验的上下文切换只需要几十到一百条指令，本身效率较高。
    // 但第一节实验中介绍的打印输出涉及到硬件的驱动和串口输出，而且是逐个字符输出，效率就低得多了。
    // 当然，后续实验我们还会讲到导致上下文切换的效率也大幅降低的其他硬件机制。

    // ex.
    // 为简化代码，本章节中我们一直在使用不安全的 Arc::get_mut_unchecked 来读写变量，它在更复杂的场景下容易导致并发冲突，实践中不建议使用。
    // 如果想要在多个执行流间安全地传递/访问数据，可以参考旧课程的 u_5_0, u_6_0, u_6_1 几节实验的代码，学习内核中的锁机制。
    axhal::misc::terminate();
}

#[allow(unused)]
fn massive_work(loops: usize) {
    let mut hash = 0usize;
    for _ in 0..loops {
        hash = (hash + 367) * 259;
    }
    println!("{hash}");
}
