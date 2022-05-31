extern "C" {
    fn c_func(p: *mut i32);
}

fn main() {
    let mut n = vec![1, 2, 3, 4, 5];
    unsafe {
        c_func(n.as_mut_ptr());
    }

    n.clear();
}
