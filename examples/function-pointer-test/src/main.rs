// The FFI is called through a callable object instead of its name

pub type Callback = unsafe extern "C" fn(*mut i32);

extern "C" {
    fn c_func(p: *mut i32);
}

pub fn run_callback(callback_object: Callback) {
    let mut n = Box::new(1);
    unsafe {
        callback_object(&mut *n);
    }

    *n = 2;
}

fn main() {
    let f = c_func;
    run_callback(f);
}
