use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[cfg(not(any(
    feature = "bluepill",
    feature = "blackpill",
    feature = "feather_nrf52840"
)))]
compile_error!("No hardware platform selected.");

#[cfg(feature = "bluepill")]
const MEMORY: &'static [u8] = include_bytes!("linker/memory-stm32f103.x");
#[cfg(feature = "blackpill")]
const MEMORY: &'static [u8] = include_bytes!("linker/memory-stm32f401.x");
#[cfg(feature = "feather_nrf52840")]
const MEMORY: &'static [u8] = include_bytes!("linker/memory-nrf52840.x");

fn main() {
    // Put the linker script somewhere the linker can find it
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::File::create(out_dir.join("memory.x"))
        .unwrap()
        .write_all(MEMORY)
        .unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=memory.x");
}
