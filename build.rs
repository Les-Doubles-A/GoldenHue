fn main() {
    println!("cargo:rerun-if-changed=GoldenHue.rc");
    println!("cargo:rerun-if-changed=GoldenHue.ico");
    #[cfg(target_os = "windows")]
    {
        let _ = embed_resource::compile("GoldenHue.rc", embed_resource::NONE).manifest_optional();
    }
}
