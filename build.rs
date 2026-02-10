use std::process::Command;

fn main() {
    // Git short SHA
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_GIT_SHA={sha}");

    // Dirty flag
    let dirty = Command::new("git")
        .args(["diff", "--quiet"])
        .status()
        .map(|s| if s.success() { "" } else { "-dirty" })
        .unwrap_or("");
    println!("cargo:rustc-env=BUILD_GIT_DIRTY={dirty}");

    // Build timestamp (UTC date only)
    let timestamp = Command::new("date")
        .args(["-u", "+%Y-%m-%d"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_TIMESTAMP={timestamp}");

    // Rerun when git state changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}
