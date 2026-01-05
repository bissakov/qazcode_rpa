extern crate embed_resource;

fn main() {
    println!("cargo:rerun-if-changed=locales");
    embed_resource::compile("../../resources/icon.rc", embed_resource::NONE);
}
