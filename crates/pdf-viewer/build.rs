use std::fs;
use std::path::Path;

fn main() {
    let workspace_root = env!("CARGO_MANIFEST_DIR")
        .split("crates")
        .next()
        .expect("Failed to determine workspace root");

    let vendor_dll = Path::new(workspace_root)
        .join("vendor")
        .join("pdfium")
        .join("pdfium.dll");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    let target_dir = out_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("Failed to determine target directory");

    let dest_dll = target_dir.join("pdfium.dll");

    if vendor_dll.exists() {
        fs::copy(&vendor_dll, &dest_dll).expect("Failed to copy pdfium.dll");
        println!(
            "cargo:warning=Copied pdfium.dll from vendor to {}",
            dest_dll.display()
        );
    } else {
        println!(
            "cargo:warning=pdfium.dll not found at {}",
            vendor_dll.display()
        );
    }

    println!("cargo:rerun-if-changed={}", vendor_dll.display());
}
