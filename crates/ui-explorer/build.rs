fn main() {
    println!("cargo:rerun-if-changed=locales");

    #[cfg(windows)]
    {
        embed_resource::compile("../../resources/icon.rc", embed_resource::NONE);
    }
}
