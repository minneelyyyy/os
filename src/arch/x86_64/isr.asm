bits 64

section .text

extern interrupt_handler

global isr6
isr6:
    push 0
    push 6
    jmp common_isr

common_isr:
    ; push rax
    ; push rbx
    ; push rcx
    ; push rdx
    ; push rbp
    ; push rdi
    ; push rsi

    ; push r8
    ; push r9
    ; push r10
    ; push r11
    ; push r12
    ; push r13
    ; push r14
    ; push r15

    mov rdi, rsp
    call interrupt_handler

.hang:
    hlt
    jmp .hang
