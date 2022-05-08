pub fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::Path::new(&out_dir);
    let out_file = out_path.join("VERSION");
    std::fs::write(out_file, version).unwrap();
}
