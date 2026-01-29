use std::process::Command;

pub fn command() -> Command {
    let mut cmd = Command::new("zellij");
    let session_args = session_args();
    if !session_args.is_empty() {
        cmd.args(session_args);
    }
    cmd
}

pub fn session_args() -> Vec<String> {
    match std::env::var("ZELLIJ_SESSION_NAME") {
        Ok(session) if !session.is_empty() => vec!["--session".to_string(), session],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut previous = Vec::new();
        for (key, value) in vars {
            previous.push((*key, std::env::var_os(key)));
            match value {
                Some(val) => std::env::set_var(key, val),
                None => std::env::remove_var(key),
            }
        }

        let result = catch_unwind(AssertUnwindSafe(f));

        for (key, value) in previous {
            match value {
                Some(val) => std::env::set_var(key, val),
                None => std::env::remove_var(key),
            }
        }

        if let Err(err) = result {
            std::panic::resume_unwind(err);
        }
    }

    #[test]
    fn session_args_empty_when_unset() {
        with_env(&[("ZELLIJ_SESSION_NAME", None)], || {
            assert!(session_args().is_empty());
        });
    }

    #[test]
    fn session_args_includes_session_name() {
        with_env(&[("ZELLIJ_SESSION_NAME", Some("test-session"))], || {
            assert_eq!(
                session_args(),
                vec!["--session".to_string(), "test-session".to_string()]
            );
        });
    }
}
