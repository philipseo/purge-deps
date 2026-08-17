use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn bin() -> Command {
    Command::new(bin_path())
}

fn bin_path() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_purge_deps") {
        return PathBuf::from(path);
    }

    // Some test runners omit CARGO_BIN_EXE_*. The binary sits next to `deps/`.
    let mut path = std::env::current_exe().expect("test executable");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push(format!("purge-deps{}", std::env::consts::EXE_SUFFIX));
    path
}

#[test]
fn help_lists_user_facing_flags() {
    let output = bin().arg("--help").output().expect("run --help");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--preset"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--replace-ignore"));
    assert!(stdout.contains("js,frontend"));
    assert!(!stdout.contains("Dash-less aliases"));
}

#[test]
fn preset_js_deletes_node_modules_but_keeps_next() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("node_modules")).unwrap();
    fs::create_dir(dir.path().join(".next")).unwrap();

    let output = bin()
        .args([
            "-p",
            dir.path().to_str().unwrap(),
            "--preset",
            "js",
            "-g",
            "false",
        ])
        .output()
        .expect("run --preset js");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!dir.path().join("node_modules").exists());
    assert!(dir.path().join(".next").exists());
}

#[test]
fn dry_run_lists_matches_without_deleting() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("node_modules")).unwrap();

    let output = bin()
        .args([
            "-p",
            dir.path().to_str().unwrap(),
            "--dry-run",
            "-g",
            "false",
        ])
        .output()
        .expect("run --dry-run");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would delete"));
    assert!(stdout.contains("PRESETS"));
    assert!(dir.path().join("node_modules").exists());
}
