#[cfg(target_arch="x86_64")]
#[path="arch/x86_64/mod.rs"]
pub mod arch;


#[macro_use]
pub mod console;
pub mod util;
pub mod driver;
pub mod memory;
pub mod interrupts;
pub mod task;
pub mod syscall;
pub mod vfs;
pub mod elf64;


pub use self::syscall::syscall_dispatch;
