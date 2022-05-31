# FFIChecker: A Static Analysis Tool For Detecting Memory Management Bugs Between Rust and C/C++

[![build](https://github.com/lizhuohua/rust-ffi-checker/actions/workflows/build.yml/badge.svg)](https://github.com/lizhuohua/rust-ffi-checker/actions/workflows/build.yml)

This tool generates and analyzes LLVM bitcode to detect potential bugs caused by incorrect use of Rust FFI.

Information about bugs detected by this tool are listed in [Trophy Case](trophy-case/README.md).

## Requirements

* Rust nightly, as specified in [rust-toolchain](rust-toolchain).
* `rustc-dev` and `llvm-tools-preview`:

    ```sh
    $ rustup component add rustc-dev llvm-tools-preview
    ```

* `LLVM 13`:

    ```sh
    # Some required libraries are included in 'libclang-common-13-dev'
    $ sudo apt-get install llvm-13-dev libclang-common-13-dev
    ```

## Build

1. Clone the repository

    ```sh
    $ git clone https://github.com/lizhuohua/rust-ffi-checker.git
    
    $ cd rust-ffi-checker
    ```

2. Build & Install

    ```sh
    # You can build and install the cargo subcommand:
    $ cargo install --path .
    
    # Or, you can only build the checker itself:
    $ cargo build
    ```

## Example

The following is a contrived example which contains a use-after-free bug. For more examples, please see [examples](examples) and [trophy-case](trophy-case).

```rust
use libc::{c_void, free};

fn main() {
    let mut n = Box::new(1);
    unsafe {
        free(&mut *n as *const _ as *mut c_void);
    }

    *n = 2;
}
```

It compiles but will crash at runtime. Our checker can detect it at compile time.

## Usage

Before using this tool, make sure your Rust project compiles without any errors or warnings.

```sh
# If you have installed the cargo subcommand:
$ cargo clean; cargo ffi-checker

# Or, you can directly run the checker binary
$ cargo clean; path/to/cargo-ffi-checker ffi-checker
```

You can also set the threshold of warnings to filter out false positives.
```sh
# Only output warnings with at least medium severity
# Available options: "high", "mid", and "low"
$ cargo clean; cargo ffi-checker -- --precision_filter mid
```

## Debug

Set `RUST_LOG` environment variable to enable logging:

```sh
# Enable all logging
$ export RUST_LOG=rust_ffi_checker

# Can also set logging level
$ export RUST_LOG=rust_ffi_checker=debug
```

For more settings, please see the documents of [env_logger](https://crates.io/crates/env_logger).

## Troubleshooting

For macOS, you may encounter `dyld: Library not loaded` error, try setting:

```sh
$ export LD_LIBRARY_PATH=$(rustc --print sysroot)/lib:$LD_LIBRARY_PATH
```

## License

See [LICENSE](LICENSE)
