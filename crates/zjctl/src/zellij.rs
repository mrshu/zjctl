use std::process::Command;

pub fn command() -> Command {
    let mut cmd = Command::new("zellij");
    if let Ok(session) = std::env::var("ZELLIJ_SESSION_NAME") {
        if !session.is_empty() {
            cmd.arg("--session").arg(session);
        }
    }
    cmd
}
