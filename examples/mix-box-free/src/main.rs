use libc::{c_void, free, malloc};
use std::mem::size_of;

#[derive(Clone)]
struct Data {
    a: Box<u32>,
}

impl Drop for Data {
    fn drop(&mut self) {
        println!("Dropping {}.", self.a);
    }
}

impl Default for Data {
    fn default() -> Self {
        println!("Initializing.");
        Self { a: Box::new(1) }
    }
}

fn main() {
    unsafe {
        let p = malloc(1 * size_of::<Data>());
        // let mut v: Vec<Data> = Vec::from_raw_parts(p as *mut Data, 100, 100);
        let v = Box::from_raw(p as *mut Data);
        // for item in v {
        //     println!("test {:?}", item.vec);
        // }
    }
}
