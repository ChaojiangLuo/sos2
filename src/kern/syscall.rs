use ::kern::console::LogLevel::*;
use ::kern::task;
use ::kern::console::{Console, tty1};
use core::sync::atomic::Ordering;

/// args: rdi, rsi, rdx, r8, r9, r10
/// rax is syscall number, and return value
pub unsafe fn syscall_entry() {
    // currently, IF disabled (msr.SFMASK)
    let rflags: usize;
    let mut rsp: usize;
    let rip: usize;
    let rax: usize;
    let rdi: usize;
    let rsi: usize;
    let rdx: usize;
    let r8: usize;
    let r9: usize;
    let r10: usize;

    asm!("
         "
         :"={r11}"(rflags),
          "={rbp}"(rsp), // send rbp + 8 == rsp because of stack frame
          "={rcx}"(rip),
          "={rax}"(rax),
          "={rdi}"(rdi),
          "={rsi}"(rsi),
          "={rdx}"(rdx),
          "={r8}"(r8),
          "={r9}"(r9),
          "={r10}"(r10)
         :
         :"memory"
         :"volatile");
    
    //printk!(Debug, "syscall: call (rip {:#x}, rsp {:#x}) {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}\n",
            //rip, rsp + 8, rax, rdi, rsi, rdx, r8, r9, r10);

    let kern_rsp: usize;
    {
        let tl = task::TaskList::get();
        let task_lock = tl.current().expect("syscall: get current task failed");
        let mut task = task_lock.write();
        kern_rsp = task.kern_stack.as_ref().map(|st| st.top()).unwrap();
        rsp += 8; // rbp + 8
        task.sysctx = task::SyscallContext {
            rip, rax, rdi, rsi, rdx, r8, r9, r10, rflags, rsp
        };
    }

    asm!("movq $0, %rsp"::"r"(kern_rsp)::"volatile");

    use x86_64::instructions::interrupts;

    interrupts::enable();
    sys_write();
    interrupts::disable();

    _syscall_return();
}

pub unsafe fn _syscall_return()
{
    let mut u_rflags: usize;
    let mut u_rsp: usize;
    let mut u_rip: usize;

    let mut rax: usize;
    let mut rdi: usize;
    let mut rsi: usize;
    let mut rdx: usize;
    let mut r8: usize;
    let mut r9: usize;
    let mut r10: usize;
    {
        let tl = task::TaskList::get();
        let task_lock = tl.current().expect("syscall: get current task failed");
        let task = task_lock.read();

        //printk!(Debug, "syscall: current task {}\n", task.pid);

        u_rflags = task.sysctx.rflags;
        u_rip = task.sysctx.rip;
        u_rsp = task.sysctx.rsp;

        rax = task.sysctx.rax;
        rdi = task.sysctx.rdi;
        rsi = task.sysctx.rsi;
        rdx = task.sysctx.rdx;
        r8 = task.sysctx.r8;
        r9 = task.sysctx.r9;
        r10 = task.sysctx.r10;
    }

    asm!("
         movq %rbx, %rbp
         movq %rbx, %rsp
         .byte 0x48
         sysret"  //0x48 = REX.W, or we can just use sysretq
         :
         :"{r11}"(u_rflags),
         "{rcx}"(u_rip),
         "{rbx}"(u_rsp),
         "{rax}"(rax),
         "{rdi}"(rdi),
         "{rsi}"(rsi),
         "{rdx}"(rdx),
         "{r8}"(r8),
         "{r9}"(r9),
         "{r10}"(r10)
         :"memory"
         :"volatile");
}

pub fn sys_write() {
    let rax: usize;
    {
        let tl = task::TaskList::get();
        let task_lock = tl.current().expect("syscall: get current task failed");
        let task = task_lock.read();
        rax = task.sysctx.rax;
    }

    let id = task::CURRENT_ID.load(Ordering::SeqCst);
    Console::with(&tty1, 19, 0, || {
        printk!(Info, "sys_write: thread {}: rax {}\n\r", id, rax);
    });
}

