use bytemuck::{Pod, Zeroable};

#[cfg(target_os = "wasi")]
pub mod call_host;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct InterestFlags: u32 {
        const READ = 0b01;
        const WRITE = 0b10;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Interest {
    pub fd: u32,
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
    pub fd: u32,
    pub interest_flags: u32,
}

impl Ready {
    pub fn flags(&self) -> InterestFlags {
        InterestFlags::from_bits_retain(self.interest_flags)
    }
}
