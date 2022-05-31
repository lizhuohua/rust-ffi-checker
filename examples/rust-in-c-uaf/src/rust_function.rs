#![crate_type = "staticlib"]

#[repr(C)]
pub struct A {
    a: i32,
    b: i32,
}

#[no_mangle]
pub extern "C" fn rust_function(obj: Box<A>) {
    println!("a={}, b={}", obj.a, obj.b);
}
