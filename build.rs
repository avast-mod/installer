fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .set("ProductName", "AVaSt Installer")
            .set("FileDescription", "AVaSt Installer")
            .compile()
            .expect("failed to embed Windows resources");
    }
}
