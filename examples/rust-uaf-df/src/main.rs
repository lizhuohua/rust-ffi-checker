fn genvec() -> Vec<u8> {
    let mut s = vec![1, 2, 3, 4, 5];
    /*fix2: let mut s = ManuallyDrop::new(String::from("a tmp string"));*/
    let ptr = s.as_mut_ptr();
    unsafe {
        let v = Vec::from_raw_parts(ptr, s.len(), s.len());
        /*fix1: mem::forget(s);*/
        return v;
        /*s is freed when the function returns*/
    }
}
fn main() {
    let v = genvec();
    assert_eq!('l' as u8, v[0]); /*use-after-free*/
    /*double free: v is released when the function returns*/
}
