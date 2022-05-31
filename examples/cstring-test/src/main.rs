extern "C" {
    fn c_func(p: *mut i8);
}

fn main() {
    let s = std::ffi::CString::new("hello!").unwrap();
    let p = s.into_raw();
    unsafe {
        c_func(p);
    }
    // let _s = unsafe { std::ffi::CString::from_raw(p) };
}
