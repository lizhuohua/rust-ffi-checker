// This extern block links to the libm library
#[link(name = "m")]
extern "C" {
    // this is a foreign function that computes cosine.
    fn cos(arg: f64) -> f64;
}

fn main() {
    let pi = 3.1415926535;
    // calling FFI is unsafe
    println!("cos(PI/2) = {:?}", unsafe { cos(pi / 2.0) });
}
