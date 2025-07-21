#![no_std]
#![no_main]

use core::panic::PanicInfo;


#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        "addi sp, sp, -8",
        "li s0, 104", 
        "sb s0, 0(sp)", // 'h'
        "li s0, 101", 
        "sb s0, 1(sp)", // 'e'
        "li s0, 108",
        "sb s0, 2(sp)", // 'l'
        "sb s0, 3(sp)", // 'l'
        "li s0, 111",
        "sb s0, 4(sp)", // 'o'
        "li s0, 10",
        "sb s0, 5(sp)", // '\n'
        "sb zero, 6(sp)", // '\0'
        "li	a0, 1",
        "mv a1, sp",
        "li	a2, 7",
        "li	a7, 64",
        "ecall", // write "hello\n"
        "mv s0, a2",
        "bne a0, a2, 1f", // check return value
        "li a0, 0",
        "li a7, 93",
        "ecall", // exit
        "1:",
        "li a0, -1",
        "li a7, 93",
        "ecall", // error exit
        options(noreturn)
    )
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
