fn main() {
    cc::Build::new()
        .file("src/c_function.c")
        .compile("c_function");
}
