//! Build script emitting custom target `cfg` flags for host JIT availability.

use std::{
    env,
    io::{self, Write},
};

fn main() -> io::Result<()> {
    let mut out = io::stdout().lock();
    out.write_all(b"cargo::rerun-if-changed=build.rs\n")?;
    out.write_all(b"cargo::rustc-check-cfg=cfg(native_jit_host)\n")?;

    let os = env::var("CARGO_CFG_TARGET_OS");
    let arch = env::var("CARGO_CFG_TARGET_ARCH");
    let supported_os = matches!(os.as_deref(), Ok("linux" | "macos" | "windows"));
    let supported_arch = matches!(arch.as_deref(), Ok("x86_64" | "aarch64" | "riscv64"));
    let jit_enabled = env::var_os("CARGO_FEATURE_NATIVE_JIT").is_some();
    let under_miri = env::var_os("CARGO_CFG_MIRI").is_some();

    if jit_enabled && !under_miri && supported_os && supported_arch {
        out.write_all(b"cargo::rustc-cfg=native_jit_host\n")?;
    }
    Ok(())
}
