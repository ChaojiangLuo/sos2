use ::kern::arch::port::Port;
use core::sync::atomic::{AtomicUsize, Ordering};
use super::idt::*;
use super::irq::PIC_CHAIN;
use spin::Mutex;
use ::kern::console::LogLevel::*;
use ::kern::console::{Console, tty1};

use ::kern::task::*;

const FREQ: u32 = 1193180;
const HZ: u32 = 100;

static TIMER_TICKS: AtomicUsize = AtomicUsize::new(0);
pub static PIT: Mutex<Timer> = Mutex::new(Timer::new());

// common ports for PIT
const TIMER_DATA: u16 = 0x40;
const TIMER_CMD: u16 = 0x43;

pub struct Timer {
    ports: [Port<u8>; 2]
}

impl Timer {
    pub const fn new() -> Timer {
        Timer {
            ports: [
                Port::new(TIMER_DATA),
                Port::new(TIMER_CMD), 
            ]
        }
    }

    pub unsafe fn init(&mut self) {
        self.ports[1].write(0x36);

        let div = FREQ / HZ;
        /*Divisor has to be sent byte-wise, so split here into upper/lower bytes.*/
        let (l, h) = (div & 0xff, (div>>8) & 0xff);

        // Send the frequency divisor.
        self.ports[0].write(l as u8);
        self.ports[0].write(h as u8);
    }

}

pub extern "C" fn timer_handler(frame: &mut ExceptionStackFrame) {
    use ::kern::console::tty1;

    unsafe { PIC_CHAIN.lock().eoi(0); }
    //printk!(Critical, "{}\n", TIMER_TICKS.load(Ordering::Acquire));
    
    let old = TIMER_TICKS.fetch_add(1, Ordering::SeqCst);
    //if (old + 1) % HZ as usize == 0 {
        //Console::with(&tty1, 0, 60, || {
            //printk!(Critical, "{}", TIMER_TICKS.load(Ordering::SeqCst));
        //});
    //}

    unsafe { sched(); }
}

