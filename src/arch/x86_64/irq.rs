use crate::printlnk;

#[repr(C)]
pub struct InterruptFrame {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,

    rsi: u64,
    rdi: u64,
    rbp: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,

    vector: u64,
    error_code: u64,

    rip: u64,
    cs: u64,
    rflags: u64,

    rsp: u64,
    ss: u64,
}

macro_rules! define_isr {
    ($i:literal, $n:ident) => {
        #[unsafe(naked)]
        pub extern "C" fn $n() {
            core::arch::naked_asm!(
                "push 0",
                concat!("push ", stringify!($i)),
                "jmp {}",
                sym common_isr
            );
        }
    };
}

macro_rules! define_isr_with_error {
    ($i:literal, $n:ident) => {
        #[unsafe(naked)]
        pub extern "C" fn $n() {
            core::arch::naked_asm!(
                concat!("push ", stringify!($i)),
                "jmp {}",
                sym common_isr
            );
        }
    };
}

define_isr!(6, invalid_opcode);
define_isr_with_error!(14, page_fault);

#[unsafe(naked)]
extern "C" fn common_isr() {
    core::arch::naked_asm!(
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rbp",
        "push rdi",
        "push rsi",

        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        "mov rcx, rsp",
        "sub rsp, 32",
        "call {}",
        "add rsp, 32",

        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",

        "pop rsi",
        "pop rdi",
        "pop rbp",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",

        // account for error code and vector
        "sub rsp, 16",

        "iretq",
        sym interrupt_handler
    );
}

extern "C" fn interrupt_handler(frame: *const InterruptFrame) {
    printlnk!("frame ptr = {:p}", frame);
    printlnk!("vector = {}", unsafe { (*frame).vector });
}
