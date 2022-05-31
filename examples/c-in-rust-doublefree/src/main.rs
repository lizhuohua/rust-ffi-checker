extern "C" {
    fn c_func(p: *mut i32);
}

fn main() {
    let mut n = Box::new(1);
    unsafe {
        c_func(&mut *n);
    }

    *n = 2;
}
