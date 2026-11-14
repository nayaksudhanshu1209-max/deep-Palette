fn main() {
    let rc = std::path::Path::new("assets/palette.rc");
    if rc.exists() {
        embed_resource::compile(rc, embed_resource::NONE);
    }
    println!("cargo:rerun-if-changed=assets/palette.rc");
    println!("cargo:rerun-if-changed=assets/palette.manifest");
}
