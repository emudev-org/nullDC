use std::process::Command;
use std::path::Path;

fn main() {
    // Only rerun build script if these files change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=debugger-ui/src");
    println!("cargo:rerun-if-changed=debugger-ui/package.json");
    println!("cargo:rerun-if-changed=debugger-ui/package-lock.json");
    println!("cargo:rerun-if-changed=debugger-ui/vite.config.ts");
    println!("cargo:rerun-if-changed=debugger-ui/tsconfig.json");

    let dist_path = Path::new("debugger-ui/dist");
    let is_ci = std::env::var("CI").is_ok();

    // If not in CI, build the debugger-UI
    if !is_ci {
        let target_arch = std::env::var("TARGET").unwrap_or_default();
        let is_wasm = target_arch.contains("wasm32");

        println!("cargo:warning=Building debugger-UI for target: {}", target_arch);

        let npm_cmd = if cfg!(target_os = "windows") { "npm.cmd" } else { "npm" };

        let status = Command::new(npm_cmd)
            .args(&["ci"])
            .current_dir("debugger-ui")
            .status()
            .expect("Failed to run npm ci");

        if !status.success() {
            panic!("npm ci failed");
        }

        let mut build_cmd = Command::new(npm_cmd);
        build_cmd
            .args(&["run", "build"])
            .current_dir("debugger-ui");

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
        println!("cargo:warning=Skipping debugger-UI build (CI environment detected)");

        // Verify that dist exists in CI
        if !dist_path.exists() {
            panic!("debugger-ui/dist not found - CI should have provided this artifact");
        }
    }
}
