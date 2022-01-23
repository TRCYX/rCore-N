#![no_std]
#![no_main]

use core::{
    fmt::Debug,
    sync::atomic::{AtomicBool, Ordering},
};
use riscv::register::uie;
use user_lib::{
    exit, fork, init_user_trap,
    ipi::{self, SenderId},
};

#[macro_use]
extern crate user_lib;
extern crate alloc;

fn ensure_result<T, E: Debug>(result: Result<T, E>, name: &str) -> T {
    match result {
        Ok(t) => t,
        Err(e) => {
            println!("{} failed with error {:?}", name, e);
            exit(1);
        }
    }
}

static RECEIVED: AtomicBool = AtomicBool::new(false);

pub fn demo() -> Result<(), isize> {
    println!("[uipi demo]");

    let _ = init_user_trap();

    let sender0 = ipi::Sender::new()?;
    let sender1 = ipi::Sender::new()?;
    let receiver0 = ipi::Receiver::new()?;
    let receiver1 = ipi::Receiver::new()?;

    sender0.connect(&receiver1)?;
    sender1.connect(&receiver0)?;

    println!(
        "sender0: {} sender1: {} receiver0: {} receiver1: {}",
        sender0.id.0, sender1.id.0, receiver0.id.0, receiver1.id.0
    );

    unsafe {
        uie::set_usoft();
    }

    let child_pid = fork();

    if child_pid == 0 {
        receiver0.listen()?;

        for i in 0..10 {
            while !RECEIVED.load(Ordering::Relaxed) {}
            println!("Child received {}", i);

            RECEIVED.store(false, Ordering::Relaxed);

            sender0.send(receiver1.id).or(Err(-1isize))?;
            #[allow(unused_must_use)]
            {
                // Not enabled
                sender0.send(receiver0.id);
            }
            println!("Child sent {}", i);
        }
    } else {
        ensure_result(receiver1.listen(), "receiver1 listen");

        for i in 0..10 {
            sender1.send(receiver0.id).or(Err(-1isize))?;
            println!("Parent sent {}", i);

            while !RECEIVED.load(Ordering::Relaxed) {}
            println!("Parent received {}", i);

            RECEIVED.store(false, Ordering::Relaxed);
        }
    }

    Ok(())
}

#[no_mangle]
pub fn main() -> i32 {
    match demo() {
        Ok(()) => 0,
        Err(e) => e as i32,
    }
}

#[no_mangle]
pub fn ipi_handler(sender_id: SenderId) {
    println!("Received ipi from sender {:?}", sender_id);
    while RECEIVED.load(Ordering::Relaxed) {}
    RECEIVED.store(true, Ordering::Relaxed);
}
