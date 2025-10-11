use std::process::Command;
use std::path::Path;

fn main() {
    // Only rerun build script if these files change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=devtools/src");
    println!("cargo:rerun-if-changed=devtools/package.json");
    println!("cargo:rerun-if-changed=devtools/package-lock.json");
    println!("cargo:rerun-if-changed=devtools/vite.config.ts");
    println!("cargo:rerun-if-changed=devtools/tsconfig.json");

    let dist_path = Path::new("devtools/dist");
    let is_ci = std::env::var("CI").is_ok();

    // If not in CI, build the debugger-UI
    if !is_ci {
        let target_arch = std::env::var("TARGET").unwrap_or_default();
        let is_wasm = target_arch.contains("wasm32");

        println!("cargo:warning=Building devtools for target: {}", target_arch);

        let npm_cmd = if cfg!(target_os = "windows") { "npm.cmd" } else { "npm" };

        let status = Command::new(npm_cmd)
            .args(&["ci"])
            .current_dir("devtools")
            .status()
            .expect("Failed to run npm ci");

        if !status.success() {
            panic!("npm ci failed");
        }

        let mut build_cmd = Command::new(npm_cmd);
        build_cmd
            .args(&["run", "build"])
            .current_dir("devtools");

        // Set environment variable for vite to know if building for wasm
        if is_wasm {
            build_cmd.env("VITE_USE_BROADCAST", "true");
        }

        let status = build_cmd.status()
            .expect("Failed to run npm build");

        if !status.success() {
            panic!("npm run build failed");
        }
    } else {
        println!("cargo:warning=Skipping devtools build (CI environment detected)");

        // Verify that dist exists in CI
        if !dist_path.exists() {
            panic!("devtools/dist not found - CI should have provided this artifact");
        }
    }
}
