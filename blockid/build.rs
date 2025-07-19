use std::env;

fn main() {
    let path = env::var("CACHE_PATH")
        .unwrap_or(String::from("/run/blockid/blockid.tab"));

    println!("cargo:warning=Using cache path: \"{path}\"");
    println!("cargo:rustc-env=CACHE_PATH={path}");
}
