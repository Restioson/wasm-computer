#[cfg(target_os = "wasi")]
#[link(wasm_import_module = "event")]
extern "C" {
    /// # Safety contract
    /// Interests array (starts at interests_ptr and is of given len) must contain [`Interest`]s with
    /// valid, open file descriptors. Ready array (starts at ready_ptr and is of given len) must be
    /// a zeroed array of [`Ready`]s.
    ///
    /// # Notes
    /// If the given [`Interest`] refers to a regular file, this function will immediately return,
    /// as regular files do not properly support non blocking mode.
    pub fn wait_until_ready(interests_ptr: i64, ready_ptr: i64, len: i64) -> i64;
}

use bytemuck::{Pod, Zeroable};

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct InterestFlags: u32 {
        const READ = 0b01;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Interest {
    pub fd: i32,
    pub interest_flags: u32,
}

impl Interest {
    pub fn flags(&self) -> InterestFlags {
        InterestFlags::from_bits_retain(self.interest_flags)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Ready {
    pub fd: i32,
    pub interest_flags: u32,
}

impl Ready {
    pub fn flags(&self) -> InterestFlags {
        InterestFlags::from_bits_retain(self.interest_flags)
    }
}
