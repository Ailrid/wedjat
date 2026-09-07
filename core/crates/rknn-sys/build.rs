use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo where to locate dynamic libraries
    println!("cargo:rustc-link-search=native=./lib");

    // Tell cargo to link against librknn3_api.so
    println!("cargo:rustc-link-lib=rknn3_api");

    // Trigger rebuild if wrapper header changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Include search paths for C headers
        .clang_arg("-Iinclude")
        // Generate C-compatible enums and layout checks
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write generated bindings to OUT_DIR
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}