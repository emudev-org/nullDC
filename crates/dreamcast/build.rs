fn main() {
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw2_stub.cc");
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw2_stub.h");

    if std::env::var("CARGO_FEATURE_REFSW2_NATIVE").is_ok() {
        cc::Build::new()
            .cpp(true)
            .file("ffi/refsw2/refsw2_stub.cc")
            .include("ffi/refsw2")
            .flag_if_supported("-std=c++17")
            .compile("refsw2");
    }
}
