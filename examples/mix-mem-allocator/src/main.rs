use libc::{c_void, free};

#[derive(Clone)]
#[repr(C)]
struct Data {
    vec: Vec<u32>,
}

impl Drop for Data {
    fn drop(&mut self) {
        println!("Dropping.");
    }
}

impl Default for Data {
    fn default() -> Self {
        println!("Initializing.");
        Self { vec: vec![1, 2, 3] }
    }
}

fn main() {
    let mut n = Box::new(Data::default());

    // Here the destructor won't be executed, so the vector is not freed
    unsafe {
        // Adding the following line to free the internal vector
        // free(n.vec.as_mut_ptr() as *mut c_void);
        free(Box::into_raw(n) as *mut c_void);
    }
}
