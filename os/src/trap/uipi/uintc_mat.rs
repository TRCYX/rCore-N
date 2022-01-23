use core::ptr::{read_volatile, write_volatile};

use crate::config::{UINTC_BASE, UINTC_MAX_RECEIVER};

use super::{ReceiverId, ReceiverUintcId, SenderId, SenderUintcId};
use lazy_static::*;
use spin::Mutex;

const SENDER_BASE: usize = 0x0;
const SENDER_STRIDE: usize = 0x2000;
const SENDER_ID_OFFSET: usize = 0x1000;
const SENDER_ENABLE_OFFSET: usize = 0x1800;
const SENDER_PENDING_OFFSET: usize = 0x1A00;
const RECEIVER_BASE: usize = 0x2000_000;
const RECEIVER_STRIDE: usize = 0x2000;
const RECEIVER_ID_OFFSET: usize = 0x1000;
const RECEIVER_ENABLE_OFFSET: usize = 0x1800;
const RECEIVER_PENDING_OFFSET: usize = 0x1A00;
const CONTEXT_BASE: usize = 0x0;
const CONTEXT_STRIDE: usize = 0x4;
pub const UINTC_SIZE: usize = 0x4000_000;

pub fn sender_addr_start(sender_uintc_id: SenderUintcId) -> usize {
    let sender_uintc_id = sender_uintc_id.0.get() as usize;
    UINTC_BASE + SENDER_BASE + SENDER_STRIDE * sender_uintc_id
}

pub fn receiver_addr_start(receiver_uintc_id: ReceiverUintcId) -> usize {
    let receiver_uintc_id = receiver_uintc_id.0.get() as usize;
    UINTC_BASE + RECEIVER_BASE + RECEIVER_STRIDE * receiver_uintc_id
}

lazy_static! {
    static ref UINTC_MAT_LOCK: Mutex<()> = Mutex::new(());
}

unsafe fn set_sender_id_unlocked(sender_uintc_id: SenderUintcId, sender_id: Option<SenderId>) {
    let sender_id = match sender_id {
        Some(v) => v.0.get(),
        None => 0,
    };

    let addr = sender_addr_start(sender_uintc_id) + SENDER_ID_OFFSET;
    write_volatile(addr as *mut u32, sender_id);
}

#[inline]
pub unsafe fn set_sender_id(sender_uintc_id: SenderUintcId, sender_id: Option<SenderId>) {
    let lock = UINTC_MAT_LOCK.lock();
    set_sender_id_unlocked(sender_uintc_id, sender_id);
    drop(lock);
}

unsafe fn set_receiver_id_unlocked(
    receiver_uintc_id: ReceiverUintcId,
    receiver_id: Option<ReceiverId>,
) {
    let receiver_id = match receiver_id {
        Some(v) => v.0.get(),
        None => 0,
    };

    let addr = receiver_addr_start(receiver_uintc_id) + RECEIVER_ID_OFFSET;
    write_volatile(addr as *mut u32, receiver_id);
}

#[inline]
pub unsafe fn set_receiver_id(receiver_uintc_id: ReceiverUintcId, receiver_id: Option<ReceiverId>) {
    let lock = UINTC_MAT_LOCK.lock();
    set_receiver_id_unlocked(receiver_uintc_id, receiver_id);
    drop(lock);
}

#[inline]
pub unsafe fn set_listening_receiver_id(
    context_id: usize,
    receiver_uintc_id: Option<ReceiverUintcId>,
) {
    let receiver_uintc_id = match receiver_uintc_id {
        Some(v) => v.0.get(),
        None => 0,
    } as u32;

    let addr = UINTC_BASE + CONTEXT_BASE + CONTEXT_STRIDE * context_id;

    let lock = UINTC_MAT_LOCK.lock();
    write_volatile(addr as *mut u32, receiver_uintc_id);
    drop(lock);
}

#[inline]
pub unsafe fn set_connected(
    sender_uintc_id: SenderUintcId,
    receiver_uintc_id: ReceiverUintcId,
    connect: bool,
) {
    let receiver_uintc_id = receiver_uintc_id.0.get() as usize;
    let word_index = receiver_uintc_id / 32;
    let bit = 1u32 << (receiver_uintc_id % 32);

    let addr = sender_addr_start(sender_uintc_id) + SENDER_ENABLE_OFFSET + word_index;

    let lock = UINTC_MAT_LOCK.lock();
    let word = read_volatile(addr as *const u32);
    write_volatile(
        addr as *mut u32,
        if connect { word | bit } else { word & !bit },
    );
    drop(lock);
}

#[inline]
pub unsafe fn drop_sender(sender_uintc_id: SenderUintcId) {
    let sender_start = sender_addr_start(sender_uintc_id);

    let lock = UINTC_MAT_LOCK.lock();

    let enable_addr = sender_start + SENDER_ENABLE_OFFSET;
    for i in 0..((UINTC_MAX_RECEIVER + 31) / 32) {
        write_volatile((enable_addr + i * 4) as *mut u32, 0);
    }

    let pending_addr = sender_start + SENDER_PENDING_OFFSET;
    for i in 0..((UINTC_MAX_RECEIVER + 31) / 32) {
        write_volatile((pending_addr + i * 4) as *mut u32, 0);
    }

    set_sender_id_unlocked(sender_uintc_id, None);

    drop(lock);
}

#[inline]
pub unsafe fn drop_receiver(receiver_uintc_id: ReceiverUintcId) {
    let receiver_start = receiver_addr_start(receiver_uintc_id);

    let lock = UINTC_MAT_LOCK.lock();

    let enable_addr = receiver_start + RECEIVER_ENABLE_OFFSET;
    for i in 0..((UINTC_MAX_RECEIVER + 31) / 32) {
        write_volatile((enable_addr + i * 4) as *mut u32, 0);
    }

    let pending_addr = receiver_start + RECEIVER_PENDING_OFFSET;
    for i in 0..((UINTC_MAX_RECEIVER + 31) / 32) {
        write_volatile((pending_addr + i * 4) as *mut u32, 0);
    }

    set_receiver_id_unlocked(receiver_uintc_id, None);

    drop(lock);
}
