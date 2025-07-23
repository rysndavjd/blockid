fn main() {
    let pointer_width = std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH")
        .expect("Failed to read target pointer width");

    if pointer_width == "32" {
        panic!("Libblockid does not intend to support 32-bit targets please use libblkid with ffi if you need 32-bit support");
    }
}