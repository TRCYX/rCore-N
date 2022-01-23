use core::{
    mem::MaybeUninit,
    num::{NonZeroU16, NonZeroU32},
    ptr::{read_volatile, write_volatile},
};

use crate::syscall::{sys_uipi_connection_ctl, sys_uipi_receiver_ctl, sys_uipi_sender_ctl};

const UINTC_BASE: usize = 0x4000_000;
const SENDER_BASE: usize = 0x0;
const SENDER_STRIDE: usize = 0x2000;
const SENDER_SEND_STATUS_OFFSET: usize = 0x0;
const RECEIVER_BASE: usize = 0x2000_000;
const RECEIVER_STRIDE: usize = 0x2000;
const RECEIVER_CLAIM_BASE: usize = 0x0;

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

macro_rules! declare_integral_newtype {
    ($type_name: ident, $base_integer_type: ty) => {
        #[repr(C)]
        #[derive(Copy, Clone, Ord, PartialOrd, Debug, Eq, PartialEq, Hash)]
        pub struct $type_name(pub $base_integer_type);

        impl From<$base_integer_type> for $type_name {
            fn from(v: $base_integer_type) -> Self {
                Self(v)
            }
        }

        impl Into<$base_integer_type> for $type_name {
            fn into(self) -> $base_integer_type {
                self.0
            }
        }
    };
}

declare_integral_newtype!(SenderId, NonZeroU32);
declare_integral_newtype!(SenderUintcId, NonZeroU16);
declare_integral_newtype!(ReceiverId, NonZeroU32);
declare_integral_newtype!(ReceiverUintcId, NonZeroU16);

pub struct Sender {
    pub id: SenderId,
    pub uintc_id: SenderUintcId,
}

pub struct Receiver {
    pub id: ReceiverId,
    pub uintc_id: ReceiverUintcId,
}

static mut LISTENING_RECEIVER_UINTC_ID: Option<ReceiverUintcId> = None;

impl Sender {
    pub fn new() -> Result<Self, isize> {
        let mut sender = MaybeUninit::<Self>::uninit();
        let sender_id_or_err = sys_uipi_sender_ctl(
            (UipiSenderCtlFlags::CREATE | UipiSenderCtlFlags::GET_INFO).bits(),
            0,
            sender.as_mut_ptr() as *mut u8,
        );
        if sender_id_or_err >= 0 {
            Ok(unsafe { sender.assume_init() })
        } else {
            Err(sender_id_or_err)
        }
    }

    pub fn set_connected(&self, receiver: &Receiver, connected: bool) -> Result<(), isize> {
        let err = sys_uipi_connection_ctl(self.id.0.get(), receiver.id.0.get(), connected);
        if err >= 0 {
            Ok(())
        } else {
            Err(err)
        }
    }

    pub fn connect(&self, receiver: &Receiver) -> Result<(), isize> {
        self.set_connected(receiver, true)
    }

    pub fn disconnect(&self, receiver: &Receiver) -> Result<(), isize> {
        self.set_connected(receiver, false)
    }

    pub fn send(&self, receiver: ReceiverId) -> Result<(), ()> {
        let sender_uintc_id = self.uintc_id.0.get() as usize;

        let addr =
            UINTC_BASE + SENDER_BASE + SENDER_STRIDE * sender_uintc_id + SENDER_SEND_STATUS_OFFSET;

        unsafe {
            write_volatile(addr as *mut NonZeroU32, receiver.0);
        }

        Ok(())
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        sys_uipi_sender_ctl(
            UipiReceiverCtlFlags::RELEASE.bits(),
            self.id.0.get(),
            0 as *mut u8,
        );
    }
}

impl Receiver {
    pub fn new() -> Result<Self, isize> {
        let mut receiver = MaybeUninit::<Self>::uninit();
        let receiver_id_or_err = sys_uipi_receiver_ctl(
            (UipiReceiverCtlFlags::CREATE | UipiReceiverCtlFlags::GET_INFO).bits(),
            0,
            receiver.as_mut_ptr() as *mut u8,
        );
        if receiver_id_or_err >= 0 {
            Ok(unsafe { receiver.assume_init() })
        } else {
            Err(receiver_id_or_err)
        }
    }

    pub fn listen(&self) -> Result<(), isize> {
        if unsafe { LISTENING_RECEIVER_UINTC_ID }.is_some() {
            println!("Already listening!");
            return Err(-1);
        }

        let receiver_id_or_err = sys_uipi_receiver_ctl(
            UipiReceiverCtlFlags::LISTEN.bits(),
            self.id.0.get(),
            0 as *mut u8,
        );
        if receiver_id_or_err >= 0 {
            unsafe {
                LISTENING_RECEIVER_UINTC_ID = Some(self.uintc_id);
            }
            Ok(())
        } else {
            Err(receiver_id_or_err)
        }
    }

    pub fn unlisten() -> Result<(), isize> {
        let receiver_id_or_err =
            sys_uipi_receiver_ctl(UipiReceiverCtlFlags::UNLISTEN.bits(), 0, 0 as *mut u8);
        if receiver_id_or_err >= 0 {
            unsafe {
                LISTENING_RECEIVER_UINTC_ID = None;
            }
            Ok(())
        } else {
            Err(receiver_id_or_err)
        }
    }

    pub fn receive() -> Option<SenderId> {
        let receiver_uintc_id = unsafe { LISTENING_RECEIVER_UINTC_ID }?;
        let receiver_uintc_id = receiver_uintc_id.0.get() as usize;

        let addr =
            UINTC_BASE + RECEIVER_BASE + RECEIVER_STRIDE * receiver_uintc_id + RECEIVER_CLAIM_BASE;
        let sender_id = unsafe { read_volatile(addr as *const u32) };

        NonZeroU32::new(sender_id).map(SenderId)
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        sys_uipi_receiver_ctl(
            UipiReceiverCtlFlags::RELEASE.bits(),
            self.id.0.get(),
            0 as *mut u8,
        );
    }
}
