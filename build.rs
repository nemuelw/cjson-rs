use std::env;
use std::path::PathBuf;

fn main() {
    if pkg_config::probe_library("cjson").is_err() {
        panic!("Could not find the cJSON library");
    }
    println!("cargo:rustc-link-lib=cjson");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
