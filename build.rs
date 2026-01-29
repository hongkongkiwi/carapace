use std::process::Command;

fn main() {
    // Capture git commit hash at compile time
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=CARAPACE_GIT_HASH={}", git_hash);

    // Capture build date
    let build_date = chrono_free_date();
    println!("cargo:rustc-env=CARAPACE_BUILD_DATE={}", build_date);

    // Re-run if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");
}

/// Get the current date in YYYY-MM-DD format without depending on chrono.
fn chrono_free_date() -> String {
    Command::new("date")
        .args(["+%Y-%m-%d"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
