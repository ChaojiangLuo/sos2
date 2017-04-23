#[macro_use] pub mod idt;
pub use self::idt::*;

use ::kern::console::LogLevel::*;
use ::kern::arch::cpu::{cs, cr2};

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.page_fault = Entry::new(cs(), define_handler_with_errno!(page_fault_handler) as u64);
        idt.breakpoint = Entry::new(cs(), define_handler!(int3_handler) as u64);
        idt.divide_by_zero = Entry::new(cs(), define_handler!(divide_by_zero_handler) as u64);

        idt
    };
}


bitflags! {
    flags PageFaultErrorCode: u64 {
        const PROTECTION_VIOLATION = 1 << 0,
        const CAUSED_BY_WRITE = 1 << 1,
        const USER_MODE = 1 << 2,
        const MALFORMED_TABLE = 1 << 3,
        const INSTRUCTION_FETCH = 1 << 4,
    }
}

extern "C" fn page_fault_handler(frame: &mut ExceptionStackFrame, err_code: u64) {
    let err = PageFaultErrorCode::from_bits(err_code).unwrap();
    printk!(Debug, "page fault! {:#?}\n\rerr code: {:#?}, cr2: {:#x}\n\r", frame, err, cr2());
    loop {
        unsafe { asm!("hlt"); }
    }
}

extern "C" fn int3_handler(frame: &mut ExceptionStackFrame) {
    printk!(Debug, "int3!! {:#?}\n\r", frame);
}

extern "C" fn divide_by_zero_handler(frame: &mut ExceptionStackFrame) {
    printk!(Debug, "divide_by_zero!! {:#?}\n\r", frame);
    loop {}
}


pub fn init() {
    IDT.load();
}

pub fn test_idt() {
    unsafe { asm!("int3"); }
    printk!(Debug, "after int3\n\r");
    //unsafe {
        //asm!("mov dx, 0; div dx":::"dx":"intel");
    //}
    //printk!(Debug, "after divide_by_zero\n\r");
    unsafe {
        let ptr = 0xdeedbeef as *mut u8;
        *ptr = 12;
    }
    printk!(Debug, "after page fault\n\r");
}
