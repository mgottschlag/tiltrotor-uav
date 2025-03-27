use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[cfg(not(any(feature = "blackpill", feature = "flightcontroller",)))]
compile_error!("No hardware platform selected.");

#[cfg(feature = "blackpill")]
const MEMORY: &'static [u8] = include_bytes!("linker/memory-stm32f411.x");
#[cfg(feature = "flightcontroller")]
const MEMORY: &'static [u8] = include_bytes!("linker/memory-stm32g491.x");

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::File::create(out_dir.join("memory.x"))
        .unwrap()
        .write_all(MEMORY)
        .unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");
}
