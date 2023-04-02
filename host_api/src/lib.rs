use bytemuck::Zeroable;
use host_api_sys as ffi;
use std::os::fd::{AsRawFd, BorrowedFd};

pub fn wait_until_ready_for_read<'a>(fds: &[BorrowedFd<'a>]) -> Vec<BorrowedFd<'a>> {
    let interests: Vec<ffi::Interest> = fds
        .iter()
        .map(|fd| ffi::Interest {
            fd: fd.as_raw_fd(),
            interest_flags: ffi::InterestFlags::READ.bits(),
        })
        .collect();

    let mut ready: Vec<ffi::Ready> = fds.iter().map(|_| ffi::Ready::zeroed()).collect();

    // SAFETY: interests and ready have equal length and are both specified correctly.
    // All interests are open and valid fds as per BorrowedFd contract.
    let n_ready = unsafe {
        host_api_sys::wait_until_ready(
            interests.as_ptr() as i64,
            ready.as_mut_ptr() as i64,
            interests.len() as i64,
        )
    };

    // SAFETY: `wait_until_ready` only returns fds given in interests, which are borrowed in param.
    // No zero fds will be returned as we only take up to n_ready fds
    ready
        .into_iter()
        .take(n_ready as usize)
        .map(|ready| unsafe { BorrowedFd::borrow_raw(ready.fd) })
        .collect()
}
