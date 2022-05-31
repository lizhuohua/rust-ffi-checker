extern "C" {
    fn c_func(p: *mut i32);
}

fn side_effect_func(p: *mut *mut i32) {
    let mut n = Box::new(1);
    unsafe {
        *p = &mut *n;
    }
    // If we don't forget it, it will be dropped here
    std::mem::forget(n);
}

fn main() {
    // Initialize a pointer `p`
    let mut p: *mut i32 = &mut 0;
    // Use side effect to change `p`, now it should point to
    // the heap memory allocated by `Box`
    side_effect_func(&mut p);
    unsafe {
        // Free `p`
        c_func(p);
        // This should be a use-after-free
        *p = 2;
    }
}
