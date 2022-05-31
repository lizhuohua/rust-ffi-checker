extern "C" {
    fn c_func(p: *mut i32);
}

fn return_value_func() -> Box<i32> {
    Box::new(1)
}

fn main() {
    let mut n = return_value_func();
    unsafe {
        c_func(&mut *n);
    }

    *n = 2;
}
