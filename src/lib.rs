#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![feature(asm)]
#![feature(range_contains)]
#![feature(alloc, collections)]
#![feature(naked_functions)]
#![feature(core_intrinsics)]
#![feature(core_slice_ext)]
// stabled since 1.17
#![feature(field_init_shorthand)]
#![no_std]

extern crate rlibc;
extern crate multiboot2;
extern crate spin;
extern crate x86_64;

extern crate kheap_allocator;
extern crate alloc;
#[macro_use] extern crate collections;

#[macro_use] extern crate bitflags;
extern crate bit_field;
#[macro_use] extern crate lazy_static;

#[macro_use] mod kern;

use kern::console as con;
use con::LogLevel::*;
use kern::driver::serial;
use kern::memory;
use kern::interrupts;
use kheap_allocator as kheap;
use kern::driver::video::framebuffer::{Framebuffer, Point, Rgba};

#[allow(dead_code)]
fn busy_wait () {
    for _ in 1..500000 {
        kern::util::cpu_relax();
    }
}

/// test rgb framebuffer drawing
fn display(fb: &mut Framebuffer) {
    use core::ptr::*;
    let vga;

    unsafe {
        vga = fb.get_mut() as *mut _;
        let mut clr: u32 = 0;

        let w = fb.width;
        for g in 0..1 {
            for i in 0..fb.height {
                fb.draw_line(Point{x:0, y:i as i32}, Point{x:w as i32-1, y: i as i32}, Rgba(clr));
                //let data = &[clr; 800];
                //let off = i * fb.width;
                //copy_nonoverlapping(data, vga.offset(off as isize) as *mut _, 1);
                let r: u32 = (256 * i / fb.height) as u32;
                clr  = (g << 8) | (r <<16);
            }

            //busy_wait();
            fb.draw_line(Point{x: 530, y: 120}, Point{x: 330, y: 10}, Rgba(0xeeeeeeee));
            fb.draw_line(Point{x: 330, y: 120}, Point{x: 530, y: 10}, Rgba(0xeeeeeeee));
            
            fb.draw_line(Point{x: 300, y: 10}, Point{x: 500, y: 100}, Rgba(0xeeeeeeee));
            fb.draw_line(Point{x: 300, y: 10}, Point{x: 400, y: 220}, Rgba(0xeeeeeeee));

            fb.draw_line(Point{x: 100, y: 220}, Point{x: 300, y: 100}, Rgba(0xeeeeeeee));
            fb.draw_line(Point{x: 100, y: 220}, Point{x: 300, y: 10}, Rgba(0xeeeeeeee));

            for r in (100..150).filter(|x| x % 5 == 0) {
                fb.draw_circle(Point{x: 200, y: 200}, r, Rgba::from(0, g as u8, 0xff));
            }

            fb.spread_circle(Point{x: 400, y: 100}, 90, Rgba::from(0, g as u8, 0xee));

            fb.draw_rect(Point{x:199, y: 199}, 202, 102, Rgba::from(0x00, g as u8, 0xff));
            fb.fill_rect(Point{x:200, y: 200}, 200, 100, Rgba::from(0x80, g as u8, 0x80));

            fb.draw_rect(Point{x:199, y: 309}, 302, 102, Rgba::from(0x00, g as u8, 0xff));
            fb.fill_rect(Point{x:200, y: 310}, 300, 100, Rgba::from(0xa0, g as u8, 0x80));

            fb.draw_rect(Point{x:199, y: 419}, 392, 102, Rgba::from(0x00, g as u8, 0xff));
            fb.fill_rect(Point{x:200, y: 420}, 390, 100, Rgba::from(0xe0, g as u8, 0x80));

            fb.draw_char(Point{x:300, y: 550}, b'A', Rgba(0x000000ff), Rgba(0x00ff0000));
            fb.draw_str(Point{x:40, y: 550}, b"Loading SOS...", Rgba(0x000000ff), Rgba(clr));

            x86_64::instructions::interrupts::disable();
            printk!(Debug, "loop {}\n\r", g);
            x86_64::instructions::interrupts::enable();
        }


    }
}

fn test_kheap_allocator() {
    let mut v = vec![1,2,3,4];
    let b = alloc::boxed::Box::new(0xcafe);
    printk!(Debug, "v = {:?}, b = {:?}\n\r", v, b);
    let vs = vec!["Loading", "SOS2", "\n\r"];
    for s in vs {
        printk!(Debug, "{} ", s);
    }

    for i in 1..0x1000 * 40 {
        v.push(i);
    }

    let range = kheap::HEAP_RANGE.try().unwrap();
    printk!(Critical, "Heap usage: {:#x}\n\r", kheap::KHEAP_ALLOCATOR.lock().current - range.start);
}

extern {
    static _start: u64;
    static _end: u64;
    static kern_stack_top: u64;
}

#[no_mangle]
pub extern fn kernel_main(mb2_header: usize) {
    unsafe { 
        let mut com1 = serial::COM1.lock();
        com1.init();
    }

    con::clear();
    printk!(Info, "Loading SOS2....\n\r");

    let mbinfo = unsafe { multiboot2::load(mb2_header) };
    printk!(Info, "{:#?}\n\r", mbinfo);

    let (pa, pe, sp_top) = unsafe {
        (&_start as *const _ as u64, &_end as *const _ as u64, &kern_stack_top as *const _ as u64)
    };
    printk!(Debug, "_start {:#X}, _end {:#X}, sp top: {:#X}\n\r", pa, pe, sp_top);

    let fb = mbinfo.framebuffer_tag().expect("framebuffer tag is unavailale");
    let mut mm = memory::init(&mbinfo);

    if cfg!(feature = "test") {
        test_kheap_allocator();
    }

    interrupts::init(&mut mm);
    if cfg!(feature = "test") {
        interrupts::test_idt();
    }

    let mut fb = Framebuffer::new(&fb);
    if cfg!(feature = "test") {
        display(&mut fb);
    }
    loop {
        kern::util::cpu_relax();
    }
}

#[lang = "eh_personality"]
extern fn eh_personality() {}

#[lang = "panic_fmt"] 
#[no_mangle] pub extern fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
	printk!(Critical, "\n\rPanic at {}:{}\n\r", file, line);
    printk!(Critical, "    {}\n\r", fmt);
    loop {
        unsafe { asm!("hlt":::: "volatile"); }
    }
}

#[lang = "eh_unwind_resume"]
#[no_mangle]
pub extern fn rust_eh_unwind_resume() {
}

/// dummy, this should never gets called
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}
