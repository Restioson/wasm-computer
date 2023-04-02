use crate::{Interest, Ready};

mod ffi {
    #[link(wasm_import_module = "event")]
    extern "C" {
        pub fn notify_ready(interests_ptr: i64, ready_ptr: i64, len: i64) -> i64;
    }
}

/// Panics if `interests.len() != ready.len()`
pub fn notify_ready(interests: &[Interest], ready: &mut [Ready]) -> usize {
    if interests.len() != ready.len() {
        panic!("interests.len() != ready.len()");
    }

    let res = unsafe {
        ffi::notify_ready(
            interests.as_ptr() as i64,
            ready.as_mut_ptr() as i64,
            interests.len() as i64,
        )
    };
    res as usize
}
