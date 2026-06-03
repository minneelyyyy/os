use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn compile_nasm(path: impl AsRef<Path>) {
    let path = path.as_ref();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let stem = path.file_stem().unwrap();
    let obj = out_dir.join(format!("{}.obj", stem.to_string_lossy()));

    let status = Command::new("nasm")
        .args([
            "-f",
            "win64",
            path.to_str().unwrap(),
            "-o",
            obj.to_str().unwrap(),
        ])
        .status()
        .expect("failed to invoke nasm");

    assert!(status.success(), "nasm failed");

    println!("cargo:rustc-link-arg={}", obj.display());
}

fn main() {
    compile_nasm("src/arch/x86_64/isr.asm");
}
