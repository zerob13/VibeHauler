use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use tempfile::tempdir;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn help_renders() {
    let output = Command::new(env!("CARGO_BIN_EXE_vhaul"))
        .arg("--help")
        .output()
        .expect("run help");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Launch the VibeHauler TUI"));
}

#[test]
fn non_tty_portable_root_renders_app_selection() {
    let output = Command::new(env!("CARGO_BIN_EXE_vhaul"))
        .arg("--portable-root")
        .arg(fixture("macos-home"))
        .arg("--no-color")
        .output()
        .expect("run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Claude Code"));
    assert!(stdout.contains("Codex"));
    assert!(stdout.contains("Gemini CLI"));
    assert!(stdout.contains("Aider"));
}

#[test]
fn tmux_tui_snapshot_flow_when_available() {
    if Command::new("tmux").arg("-V").output().is_err() {
        return;
    }

    let temp = tempdir().expect("tempdir");
    let fake_home = temp.path().join("macos-home");
    copy_dir(&fixture("macos-home"), &fake_home).expect("copy fixture");
    let session = format!("vhaul-test-{}", std::process::id());
    let exe = env!("CARGO_BIN_EXE_vhaul");

    run_tmux(["new-session", "-d", "-s", &session]);
    run_tmux([
        "send-keys",
        "-t",
        &session,
        &format!("{} --portable-root {} --no-color", exe, fake_home.display()),
        "Enter",
    ]);
    thread::sleep(Duration::from_millis(800));
    let app_selection = capture(&session);
    assert!(app_selection.contains("Select apps to inspect"));
    assert!(app_selection.contains("Claude Code"));

    run_tmux(["send-keys", "-t", &session, "Enter"]);
    thread::sleep(Duration::from_millis(800));
    let safe_cleanup = capture(&session);
    assert!(safe_cleanup.contains("Safe Cleanup"));
    assert!(safe_cleanup.contains("cache") || safe_cleanup.contains("logs"));

    run_tmux(["send-keys", "-t", &session, "Enter"]);
    thread::sleep(Duration::from_millis(800));
    let confirm = capture(&session);
    assert!(confirm.contains("Confirm safe cleanup"));

    run_tmux(["send-keys", "-t", &session, "y"]);
    thread::sleep(Duration::from_millis(800));
    let session_overview = capture(&session);
    assert!(session_overview.contains("Session Data"));

    run_tmux(["send-keys", "-t", &session, "s"]);
    thread::sleep(Duration::from_millis(300));
    let final_summary = capture(&session);
    assert!(final_summary.contains("Final Summary"));

    run_tmux(["kill-session", "-t", &session]);
}

fn run_tmux<const N: usize>(args: [&str; N]) {
    let status = Command::new("tmux").args(args).status().expect("run tmux");
    assert!(status.success());
}

fn capture(session: &str) -> String {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session, "-p"])
        .output()
        .expect("capture pane");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("utf8")
}

fn copy_dir(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.is_dir() {
            copy_dir(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}
