use cmake::Config;
use std::env;
use std::path::PathBuf;

fn main() {
    let dst = Config::new("c")
        .define("CMAKE_BUILD_TYPE", "Release")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=ui_automation_native");
    println!("cargo:rustc-link-lib=dylib=user32");
    println!("cargo:rustc-link-lib=dylib=kernel32");
    println!("cargo:rustc-link-lib=dylib=ole32");
    println!("cargo:rustc-link-lib=dylib=oleaut32");
    println!("cargo:rustc-link-lib=dylib=UIAutomationCore");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=c/CMakeLists.txt");
    println!("cargo:rerun-if-changed=c/src");
    println!("cargo:rerun-if-changed=c/include");

    println!("cargo:warning=Build output: {}", out_path.display());
}
