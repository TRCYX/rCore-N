#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(asm)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(map_first_last)]

extern crate alloc;
extern crate rv_plic;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;

use crate::{mm::init_kernel_space, sbi::send_ipi};

#[macro_use]
mod console;
mod config;
mod console_blog;
mod fs;
mod lang_items;
mod loader;
mod logger;
mod mm;
mod plic;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;
#[macro_use]
mod uart;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.asm"));

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

#[no_mangle]
pub fn rust_main(hart_id: usize) -> ! {
    if hart_id == 0 {
        clear_bss();
        mm::init();
        uart::init();
        logger::init();
        debug!("[kernel {}] Hello, world!", hart_id);

        extern "C" {
            fn boot_stack();
            fn boot_stack_top();
        }

        debug!(
            "boot_stack {:x} top {:x}",
            boot_stack as usize, boot_stack_top as usize
        );

        mm::remap_test();

        plic::init();
        plic::init_hart(hart_id);

        trap::init();

        debug!("trying to add initproc");
        task::add_initproc();
        println!("initproc added to task manager!");

        unsafe {
            let satp: usize;
            asm!("csrr {}, satp", out(reg) satp);
            println_hart!("satp {}", hart_id, satp);
        }

        for i in 1..4 {
            debug!("[kernel {}] Start {}", hart_id, i);
            let mask: usize = 1 << i;
            send_ipi(&mask as *const _ as usize);
        }
    } else {
        let hart_id = task::hart_id();

        init_kernel_space();

        unsafe {
            let satp: usize;
            asm!("csrr {}, satp", out(reg) satp);
            println_hart!("satp {}", hart_id, satp);
        }
        trap::init();
    }

    println_hart!("Hello", hart_id);

    timer::set_next_trigger();
    // loader::list_apps();

    task::run_tasks();
    panic!("Unreachable in rust_main!");
}
