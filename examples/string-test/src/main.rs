extern "C" {
    fn c_func(p: *mut u8);
}

fn main() {
    let mut s = String::from("hello!");
    unsafe {
        c_func(s.as_mut_ptr());
    }

    s.clear();
}
