use std::rc::Rc;

extern "C" {
    fn c_func(p: *mut i32);
}

fn main() {
    let mut n = Rc::new(Box::new(1));
    unsafe {
        c_func(&mut **Rc::make_mut(&mut n));
    }

    // *n = 2;
}
