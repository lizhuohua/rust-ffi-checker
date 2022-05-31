// Some utility functions shared by multiple components

use rustc_demangle::demangle;

/// Copied from Miri
/// Returns the "default sysroot" if no `--sysroot` flag is set.
/// Should be a compile-time constant.
pub fn compile_time_sysroot() -> Option<String> {
    if option_env!("RUSTC_STAGE").is_some() {
        // This is being built as part of rustc, and gets shipped with rustup.
        // We can rely on the sysroot computation in librustc.
        return None;
    }
    // For builds outside rustc, we need to ensure that we got a sysroot
    // that gets used as a default.  The sysroot computation in librustc would
    // end up somewhere in the build dir.
    // Taken from PR <https://github.com/Manishearth/rust-clippy/pull/911>.
    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    Some(match (home, toolchain) {
        (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
        _ => option_env!("RUST_SYSROOT")
            .expect("To build Miri without rustup, set the `RUST_SYSROOT` env var at build time")
            .to_owned(),
    })
}

/// Gets the value of a `name`.
/// For example, get_arg_flag_value("--manifest-path")
/// Supports two styles: `--name value` or `--name=value`
pub fn get_arg_flag_value(name: &str) -> Option<String> {
    // Stop searching at `--`.
    let mut args = std::env::args().take_while(|val| val != "--");
    loop {
        let arg = match args.next() {
            Some(arg) => arg,
            None => return None,
        };
        if !arg.starts_with(name) {
            continue;
        }
        // Strip leading `name`.
        let suffix = &arg[name.len()..];
        if suffix.is_empty() {
            // This argument is exactly `name`; the next one is the value.
            return args.next();
        } else if suffix.starts_with('=') {
            // This argument is `name=value`; get the value.
            // Strip leading `=`.
            return Some(suffix[1..].to_owned());
        }
    }
}

/// Determines whether a flag `name` is present before `--`.
/// For example, has_arg_flag("-v")
pub fn has_arg_flag(name: &str) -> bool {
    let mut args = std::env::args().take_while(|val| val != "--");
    args.any(|val| val == name)
}

/// Demangle a name
pub fn demangle_name(name: &String) -> String {
    format!("{:#}", demangle(name))
}
