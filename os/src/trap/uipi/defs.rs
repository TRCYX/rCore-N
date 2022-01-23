use core::{
    convert::TryInto,
    fmt::Display,
    num::{NonZeroU16, NonZeroU32},
    ops::AddAssign,
};

use alloc::vec::Vec;
use lazy_static::*;
use num::{Integer, One};
use spin::Mutex;

use crate::{trap::uipi::set_sender_id, config::{UINTC_MAX_SENDER, UINTC_MAX_RECEIVER}};

use super::{drop_receiver, drop_sender, set_receiver_id};

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

pub struct StackIntegerAllocator<I: Integer + AddAssign + Display + Copy> {
    current: I,
    end: I,
    recycled: Vec<I>,
}

impl<I: Integer + AddAssign + Display + Copy> StackIntegerAllocator<I> {
    pub fn new(l: I, r: I) -> Self {
        Self {
            current: l,
            end: r,
            recycled: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> Option<I> {
        if let Some(t) = self.recycled.pop() {
            Some(t.into())
        } else if self.current == self.end {
            None
        } else {
            let result = self.current;
            self.current += One::one();
            Some(result)
        }
    }

    pub fn dealloc(&mut self, i: I) {
        // validity check
        #[cfg(debug_assertions)]
        if i >= self.current || self.recycled.iter().any(|v| *v == i) {
            panic!("Frame ppn={:#} has not been allocated!", i);
        }
        // recycle
        self.recycled.push(i);
    }
}

lazy_static! {
    pub static ref SENDER_ID_ALLOCATOR: Mutex<StackIntegerAllocator<u32>> =
        Mutex::new(StackIntegerAllocator::new(1, UINTC_MAX_SENDER as u32));
    pub static ref SENDER_UINTC_ID_ALLOCATOR: Mutex<StackIntegerAllocator<u16>> =
        Mutex::new(StackIntegerAllocator::new(1, UINTC_MAX_SENDER as u16));
    pub static ref RECEIVER_ID_ALLOCATOR: Mutex<StackIntegerAllocator<u32>> =
        Mutex::new(StackIntegerAllocator::new(1, UINTC_MAX_RECEIVER as u32));
    pub static ref RECEIVER_UINTC_ID_ALLOCATOR: Mutex<StackIntegerAllocator<u16>> =
        Mutex::new(StackIntegerAllocator::new(1, UINTC_MAX_RECEIVER as u16));
}

impl SenderId {
    pub fn alloc() -> Option<Self> {
        let v = SENDER_ID_ALLOCATOR.lock().alloc()?;
        Some(Self(v.try_into().ok()?))
    }

    pub fn dealloc(self) {
        SENDER_ID_ALLOCATOR.lock().dealloc(self.0.get())
    }
}

impl SenderUintcId {
    pub fn alloc() -> Option<Self> {
        let v = SENDER_UINTC_ID_ALLOCATOR.lock().alloc()?;
        Some(Self(v.try_into().ok()?))
    }

    pub fn dealloc(self) {
        SENDER_UINTC_ID_ALLOCATOR.lock().dealloc(self.0.get())
    }
}

impl ReceiverId {
    pub fn alloc() -> Option<Self> {
        let v = RECEIVER_ID_ALLOCATOR.lock().alloc()?;
        Some(Self(v.try_into().ok()?))
    }

    pub fn dealloc(self) {
        RECEIVER_ID_ALLOCATOR.lock().dealloc(self.0.get())
    }
}

impl ReceiverUintcId {
    pub fn alloc() -> Option<Self> {
        let v = RECEIVER_UINTC_ID_ALLOCATOR.lock().alloc()?;
        Some(Self(v.try_into().ok()?))
    }

    pub fn dealloc(self) {
        RECEIVER_UINTC_ID_ALLOCATOR.lock().dealloc(self.0.get())
    }
}

#[repr(C)] // for handling to the user
#[derive(Clone, Copy)]
pub struct SenderInfo {
    pub id: SenderId,
    pub uintc_id: SenderUintcId,
}

/// RAII struct that actually owns the SenderId and SenderUintcId.
pub struct SenderHandle(pub SenderInfo);

impl SenderHandle {
    pub fn new() -> Option<Self> {
        let id = SenderId::alloc()?;
        let uintc_id = SenderUintcId::alloc()?;

        unsafe {
            set_sender_id(uintc_id, Some(id));
        }

        Some(Self(SenderInfo { id, uintc_id }))
    }
}

impl Drop for SenderHandle {
    fn drop(&mut self) {
        unsafe {
            drop_sender(self.0.uintc_id);
        }

        self.0.id.dealloc();
        self.0.uintc_id.dealloc();
    }
}

#[repr(C)] // for handling to the user
#[derive(Clone, Copy)]
pub struct ReceiverInfo {
    pub id: ReceiverId,
    pub uintc_id: ReceiverUintcId,
}

/// RAII struct that actually owns the ReceiverId and ReceiverUintcId.
pub struct ReceiverHandle(pub ReceiverInfo);

impl ReceiverHandle {
    pub fn new() -> Option<Self> {
        let id = ReceiverId::alloc()?;
        let uintc_id = ReceiverUintcId::alloc()?;

        unsafe {
            set_receiver_id(uintc_id, Some(id));
        }

        Some(Self(ReceiverInfo { id, uintc_id }))
    }
}

impl Drop for ReceiverHandle {
    fn drop(&mut self) {
        unsafe {
            drop_receiver(self.0.uintc_id);
        }

        self.0.id.dealloc();
        self.0.uintc_id.dealloc();
    }
}
