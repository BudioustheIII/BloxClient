fn main() {
    // Without this, Cargo can keep embedding a stale app icon after the files change.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
