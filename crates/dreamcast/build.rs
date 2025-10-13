fn main() {
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw2_stub.cc");
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw_lists.cc");
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw_tile.cc");
    println!("cargo:rerun-if-changed=ffi/refsw2/TexUtils.cc");
    println!("cargo:rerun-if-changed=ffi/refsw2/TexUtils.h");
    println!("cargo:rerun-if-changed=ffi/refsw2/core_structs.h");
    println!("cargo:rerun-if-changed=ffi/refsw2/gentable.h");
    println!("cargo:rerun-if-changed=ffi/refsw2/pvr_mem.h");
    println!("cargo:rerun-if-changed=ffi/refsw2/pvr_regs.h");
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw2_stub.h");
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw_lists.h");
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw_lists_regtypes.h");
    println!("cargo:rerun-if-changed=ffi/refsw2/refsw_tile.h");

    if std::env::var("CARGO_FEATURE_REFSW2_NATIVE").is_ok() {
        cc::Build::new()
            .cpp(true)
            .file("ffi/refsw2/refsw2_stub.cc")
            .file("ffi/refsw2/refsw_lists.cc")
            .file("ffi/refsw2/refsw_tile.cc")
            .file("ffi/refsw2/TexUtils.cc")
            .include("ffi/refsw2")
            .flag_if_supported("-std=c++17")
            .compile("refsw2");
    }
}
