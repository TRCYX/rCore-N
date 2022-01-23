use core::{convert::TryInto, mem::size_of, num::NonZeroU32};

use alloc::sync::Arc;

use crate::{
    config::PAGE_SIZE,
    mm::{translated_byte_buffer, UserBuffer},
    task::{current_task, hart_id},
    trap::{
        uipi::{
            receiver_addr_start, sender_addr_start, set_connected, set_listening_receiver_id,
            ReceiverHandle, ReceiverId, ReceiverInfo, SenderHandle, SenderId, SenderInfo,
        },
        UserTrapInfo,
    },
};
use spin::Mutex;

bitflags! {
    pub struct UipiSenderCtlFlags: usize {
        const CREATE = 1;
        const RELEASE = 2;
        const GET_INFO = 4;
    }
}

bitflags! {
    pub struct UipiReceiverCtlFlags: usize {
        const CREATE = 1;
        const RELEASE = 2;
        const GET_INFO = 4;
        const LISTEN = 8;
        const UNLISTEN = 16;
    }
}

fn find_sender_info(
    user_trap_info: &mut UserTrapInfo,
    sender_id: usize,
) -> Result<SenderInfo, isize> {
    let sender_id: u32 = sender_id.try_into().or(Err(-1isize))?;
    let sender_id: NonZeroU32 = sender_id.try_into().or(Err(-1isize))?;
    let sender_id: SenderId = sender_id.into();

    Ok(user_trap_info
        .uipi_senders
        .get(&sender_id)
        .ok_or(-1isize)?
        .lock()
        .0)
}

fn find_receiver_info(
    user_trap_info: &mut UserTrapInfo,
    receiver_id: usize,
) -> Result<ReceiverInfo, isize> {
    let receiver_id: u32 = receiver_id.try_into().or(Err(-1isize))?;
    let receiver_id: NonZeroU32 = receiver_id.try_into().or(Err(-1isize))?;
    let receiver_id: ReceiverId = receiver_id.into();

    Ok(user_trap_info
        .uipi_receivers
        .get(&receiver_id)
        .ok_or(-1isize)?
        .lock()
        .0)
}

#[inline]
fn uipi_sender_ctl_impl(flags: usize, sender_id: usize, buf: *mut u8) -> Result<usize, isize> {
    let flags = UipiSenderCtlFlags::from_bits(flags).ok_or(-1isize)?;

    let task = current_task().unwrap();
    let mut inner_mutex_guard = task.acquire_inner_lock();
    let inner = &mut *inner_mutex_guard;
    let token = inner.get_user_token();
    let memory_set = &mut inner.memory_set;
    let user_trap_info = inner.user_trap_info.as_mut().ok_or(-1isize)?;

    let sender_info = if flags.contains(UipiSenderCtlFlags::CREATE) {
        let sender_handle = SenderHandle::new().ok_or(-1isize)?;
        let sender_info = sender_handle.0;
        let sender_id = sender_info.id;

        if memory_set
            .mmio_map(sender_addr_start(sender_info.uintc_id), PAGE_SIZE, 0b11)
            .is_err()
        {
            drop(sender_handle);
            return Err(-1);
        }

        let arc = Arc::new(Mutex::new(sender_handle));
        if user_trap_info.uipi_senders.insert(sender_id, arc).is_some() {
            panic!("Allocated an existing SenderId {:?}!", sender_id);
        }

        sender_info
    } else {
        find_sender_info(user_trap_info, sender_id)?
    };

    if flags.contains(UipiSenderCtlFlags::GET_INFO) {
        let buffers =
            translated_byte_buffer(token, buf, size_of::<SenderInfo>()).or(Err(-1isize))?;
        let mut user_buffer = UserBuffer::new(buffers);
        user_buffer.write(&sender_info).unwrap();
    }

    if flags.contains(UipiSenderCtlFlags::RELEASE) {
        if memory_set
            .mmio_unmap(sender_addr_start(sender_info.uintc_id), PAGE_SIZE)
            .is_err()
        {
            warn!(
                "UINTC Sender page for {:?} / {:?} already unmapped!",
                sender_info.id, sender_info.uintc_id
            );
        }

        if user_trap_info
            .uipi_senders
            .remove(&sender_info.id)
            .is_none()
        {
            panic!("Deleted an nonexistent SenderId {:?}!", sender_info.id);
        }
    }

    Ok(sender_info.id.0.get() as usize)
}

#[inline]
fn uipi_receiver_ctl_impl(flags: usize, receiver_id: usize, buf: *mut u8) -> Result<usize, isize> {
    let flags = UipiReceiverCtlFlags::from_bits(flags).ok_or(-1isize)?;

    let task = current_task().unwrap();
    let mut inner_mutex_guard = task.acquire_inner_lock();
    let inner = &mut *inner_mutex_guard;
    let token = inner.get_user_token();
    let memory_set = &mut inner.memory_set;
    let user_trap_info = inner.user_trap_info.as_mut().ok_or(-1isize)?;

    fn unlisten(
        user_trap_info: &mut UserTrapInfo,
        result: Option<ReceiverId>,
    ) -> Result<usize, isize> {
        user_trap_info.listening_receiver_uintc_id = None;
        unsafe {
            set_listening_receiver_id(hart_id(), None);
        }
        Ok(match result {
            Some(id) => id.0.get() as usize,
            None => 0,
        })
    }

    // Only in this case, we do not bother with receiver_id
    if flags == UipiReceiverCtlFlags::UNLISTEN {
        return unlisten(user_trap_info, None);
    }

    let receiver_info = if flags.contains(UipiReceiverCtlFlags::CREATE) {
        let receiver_handle = ReceiverHandle::new().ok_or(-1isize)?;
        let receiver_info = receiver_handle.0;
        let receiver_id = receiver_info.id;

        if memory_set
            .mmio_map(receiver_addr_start(receiver_info.uintc_id), PAGE_SIZE, 0b11)
            .is_err()
        {
            drop(receiver_handle);
            return Err(-1);
        }

        let arc = Arc::new(Mutex::new(receiver_handle));
        if user_trap_info
            .uipi_receivers
            .insert(receiver_id, arc)
            .is_some()
        {
            panic!("Allocated an existing ReceiverId {:?}!", receiver_id);
        }

        receiver_info
    } else {
        find_receiver_info(user_trap_info, receiver_id)?
    };

    if flags.contains(UipiReceiverCtlFlags::GET_INFO) {
        let buffers =
            translated_byte_buffer(token, buf, size_of::<ReceiverInfo>()).or(Err(-1isize))?;
        let mut user_buffer = UserBuffer::new(buffers);
        user_buffer.write(&receiver_info).unwrap();
    }

    if flags.contains(UipiReceiverCtlFlags::LISTEN) {
        unsafe {
            set_listening_receiver_id(hart_id(), Some(receiver_info.uintc_id));
        }
        user_trap_info.listening_receiver_uintc_id = Some(receiver_info.uintc_id);
    }

    if flags.contains(UipiReceiverCtlFlags::UNLISTEN) {
        unlisten(user_trap_info, Some(receiver_info.id))?;
    }

    if flags.contains(UipiReceiverCtlFlags::RELEASE) {
        if user_trap_info.listening_receiver_uintc_id == Some(receiver_info.uintc_id) {
            unsafe {
                set_listening_receiver_id(hart_id(), None);
            }
        }

        if memory_set
            .mmio_unmap(receiver_addr_start(receiver_info.uintc_id), PAGE_SIZE)
            .is_err()
        {
            warn!(
                "UINTC Receiver page for {:?} / {:?} already unmapped!",
                receiver_info.id, receiver_info.uintc_id
            );
        }

        if user_trap_info
            .uipi_receivers
            .remove(&receiver_info.id)
            .is_none()
        {
            panic!("Deleted an nonexistent ReceiverId {:?}!", receiver_info.id);
        }
    }

    Ok(receiver_info.id.0.get() as usize)
}

#[inline]
fn uipi_connection_ctl_impl(
    sender_id: usize,
    receiver_id: usize,
    connected: bool,
) -> Result<usize, isize> {
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();

    let user_trap_info = inner.user_trap_info.as_mut().ok_or(-1isize)?;

    let sender_info = find_sender_info(user_trap_info, sender_id)?;
    let receiver_info = find_receiver_info(user_trap_info, receiver_id)?;

    unsafe {
        set_connected(sender_info.uintc_id, receiver_info.uintc_id, connected);
    }

    Ok(0)
}

/// Convert from a Rust `Result` to an `isize` that can be returned from as syscall.
#[inline]
fn return_from_result(r: Result<usize, isize>) -> isize {
    match r {
        Ok(u) => u as isize,
        Err(e) => e,
    }
}

pub fn sys_uipi_sender_ctl(flags: usize, sender_id: usize, buf: *mut u8) -> isize {
    return_from_result(uipi_sender_ctl_impl(flags, sender_id, buf))
}

pub fn sys_uipi_receiver_ctl(flags: usize, receiver_id: usize, buf: *mut u8) -> isize {
    return_from_result(uipi_receiver_ctl_impl(flags, receiver_id, buf))
}

pub fn sys_uipi_connection_ctl(sender_id: usize, receiver_id: usize, connected: bool) -> isize {
    return_from_result(uipi_connection_ctl_impl(sender_id, receiver_id, connected))
}
