// Mute the warning when using `Box` in FFI
#![allow(improper_ctypes)]
use libc;

#[repr(C)]
pub struct CStruct {
    pub x: libc::c_int,
    pub y: libc::c_int,
}

extern "C" {
    fn c_function(c_obj: *mut CStruct);
}

fn main() {
    // Rust allocates memory here
    let c_obj = Box::new(CStruct { x: 1, y: 2 });
    unsafe {
        // Rust passes the ownership to a C function.
        // Memory leaks since this function does not deallocate memory
        c_function(Box::into_raw(c_obj));
    }
}
