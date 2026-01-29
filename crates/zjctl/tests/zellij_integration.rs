use serde::Deserialize;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
struct PaneInfo {
    id: String,
    pane_type: String,
}

fn zjctl_bin() -> &'static str {
    env!("CARGO_BIN_EXE_zjctl")
}

fn should_run() -> bool {
    std::env::var("ZJCTL_INTEGRATION").is_ok() && std::env::var("ZELLIJ_SESSION_NAME").is_ok()
}

fn list_panes() -> Vec<PaneInfo> {
    let output = Command::new(zjctl_bin())
        .args(["panes", "ls", "--json"])
        .output()
        .expect("failed to run zjctl panes ls");
    assert!(
        output.status.success(),
        "zjctl panes ls failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("failed to parse panes json")
}

fn run_zellij(args: &[&str]) {
    let output = Command::new("zellij")
        .args(args)
        .output()
        .expect("failed to run zellij");
    assert!(
        output.status.success(),
        "zellij command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn find_new_terminal(before: &[PaneInfo], after: &[PaneInfo]) -> Option<String> {
    let before_ids: std::collections::HashSet<_> = before.iter().map(|p| p.id.as_str()).collect();
    after
        .iter()
        .filter(|p| p.pane_type == "terminal")
        .find(|p| !before_ids.contains(p.id.as_str()))
        .map(|p| p.id.clone())
}

#[test]
fn pane_send_is_single_delivery() {
    if !should_run() {
        eprintln!("skipping integration test (set ZJCTL_INTEGRATION=1 and ZELLIJ_SESSION_NAME)");
        return;
    }

    let session = std::env::var("ZELLIJ_SESSION_NAME").expect("missing ZELLIJ_SESSION_NAME");
    let marker = format!("MARKER_{}", uuid::Uuid::new_v4().simple());
    let pane_name = format!("zjctl-integ-{}", uuid::Uuid::new_v4().simple());

    let before = list_panes();

    run_zellij(&[
        "--session",
        session.as_str(),
        "action",
        "new-pane",
        "--name",
        pane_name.as_str(),
        "--",
        "fish",
    ]);

    let start = Instant::now();
    let timeout = Duration::from_secs(60);
    let pane_id = loop {
        let after = list_panes();
        if let Some(id) = find_new_terminal(&before, &after) {
            break id;
        }
        if start.elapsed() >= timeout {
            panic!("timed out waiting for new pane");
        }
        sleep(Duration::from_millis(500));
    };

    let selector = format!("id:{pane_id}");

    let send = Command::new(zjctl_bin())
        .args([
            "pane",
            "send",
            "--pane",
            selector.as_str(),
            "--enter=false",
            "--",
            marker.as_str(),
        ])
        .output()
        .expect("failed to run zjctl pane send");
    assert!(
        send.status.success(),
        "zjctl pane send failed: {}",
        String::from_utf8_lossy(&send.stderr)
    );

    sleep(Duration::from_secs(1));

    let capture = Command::new(zjctl_bin())
        .args(["pane", "capture", "--pane", selector.as_str()])
        .output()
        .expect("failed to run zjctl pane capture");
    assert!(
        capture.status.success(),
        "zjctl pane capture failed: {}",
        String::from_utf8_lossy(&capture.stderr)
    );

    let output = String::from_utf8_lossy(&capture.stdout);
    let count = output.matches(&marker).count();
    assert_eq!(count, 1, "expected marker to appear once, got {count}");

    let _ = Command::new(zjctl_bin())
        .args(["pane", "close", "--pane", selector.as_str(), "--force"])
        .status();
}
