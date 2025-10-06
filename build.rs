use std::process::Command;
use std::path::Path;

fn main() {
    // Only build debugger-UI on native targets
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dist_path = Path::new("debugger-ui/dist");
        let is_ci = std::env::var("CI").is_ok();

        // If not in CI, build the debugger-UI
        if !is_ci {
            println!("cargo:warning=Building debugger-UI");

            let status = Command::new("npm")
                .args(&["ci"])
                .current_dir("debugger-ui")
                .status()
                .expect("Failed to run npm ci");

            if !status.success() {
                panic!("npm ci failed");
            }

            let status = Command::new("npm")
                .args(&["run", "build"])
                .current_dir("debugger-ui")
                .status()
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

        // Tell cargo to rerun if debugger-ui source files change
        println!("cargo:rerun-if-changed=debugger-ui/src");
        println!("cargo:rerun-if-changed=debugger-ui/package.json");
        println!("cargo:rerun-if-changed=debugger-ui/package-lock.json");
        println!("cargo:rerun-if-changed=debugger-ui/vite.config.ts");
        println!("cargo:rerun-if-changed=debugger-ui/tsconfig.json");

        // Also rerun if dist changes (e.g., manually built)
        println!("cargo:rerun-if-changed=debugger-ui/dist");
    }
}
