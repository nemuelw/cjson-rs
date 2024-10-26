use std::env;
use std::path::PathBuf;

fn main() {
    let include_path =
        env::var("CJSON_INCLUDE_PATH").expect("Environment variable CJSON_INCLUDE_PATH not found");
    let lib_path =
        env::var("CJSON_LIB_PATH").expect("Environment variable CJSON_LIB_PATH not found");

    println!("cargo:rustc-link-search={}", lib_path);
    println!("cargo:rustc-link-lib=cjson");

    let bindings = bindgen::Builder::default()
        .header(format!("{}/cJSON.h", include_path))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
