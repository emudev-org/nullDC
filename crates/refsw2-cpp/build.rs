fn main() {
    // Build C++ backend
    cc::Build::new()
        .cpp(true)
        .file("ffi/refsw2_stub.cc")
        .file("ffi/refsw_lists.cc")
        .file("ffi/refsw_tile.cc")
        .file("ffi/TexUtils.cc")
        .flag_if_supported("-std=c++20")
        .flag_if_supported("/std:c++20")
        .flag_if_supported("-O3")
        .flag_if_supported("/O2")
        .compile("refsw2_cpp");

    println!("cargo:rerun-if-changed=ffi/");
}
