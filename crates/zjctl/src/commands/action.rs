//! Pass-through to zellij action

use std::process::Command;

pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        let status = Command::new("zellij").args(["action", "--help"]).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("zellij action exited with code: {:?}", status.code()).into());
    }

    let status = Command::new("zellij").arg("action").args(args).status()?;

    if !status.success() {
        return Err(format!("zellij action exited with code: {:?}", status.code()).into());
    }

    Ok(())
}
