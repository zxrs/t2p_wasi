use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    env::set_var("AR", "llvm-ar");
    env::set_var(
        "CFLAGS",
        "--sysroot=~/repo/wasi-sdk-20.0/share/wasi-sysroot",
    );

    let configure = Path::new("./libtiff/configure");
    if !configure.exists() {
        Command::new("sh")
            .current_dir("./libtiff")
            .arg("./autogen.sh")
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }
    let libport_config = Path::new("./libtiff/port/libport_config.h");
    if !libport_config.exists() {
        Command::new("sh")
            .current_dir("./libtiff")
            .arg("./configure")
            .arg("--target=wasm32-wasi")
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }

    let p = fs::read_dir("./libtiff/libtiff").unwrap().filter_map(|e| {
        let e = e.ok()?;
        let file_name = e.file_name();
        let file_name = file_name.to_str()?;
        if e.file_type().ok()?.is_file()
            && file_name.starts_with("tif_")
            && file_name.ends_with(".c")
            && !file_name.contains("win32")
        {
            return Some(e.path());
        }
        None
    });

    cc::Build::new()
        .file("./libtiff/tools/tiff2pdf.c")
        .files(p)
        .flag("-Wno-unused-function")
        .flag("-Wno-shift-op-parentheses")
        .flag("-Wno-format")
        .define("_WASI_EMULATED_MMAN", None)
        .include("./libtiff/port")
        .include("./libtiff/libtiff")
        .include("../../repo/wasi-sdk-20.0/share/wasi-sysroot/include")
        .compile("tiff");
}
