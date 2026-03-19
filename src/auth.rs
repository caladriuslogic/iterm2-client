use crate::error::{Error, Result};
use std::env;
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Credentials {
    pub cookie: String,
    pub key: String,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("cookie", &"[redacted]")
            .field("key", &"[redacted]")
            .finish()
    }
}

pub trait AppleScriptRunner: Send + Sync {
    fn run_osascript(&self, script: &str) -> std::result::Result<String, String>;
}

pub struct OsascriptRunner;

impl AppleScriptRunner for OsascriptRunner {
    fn run_osascript(&self, script: &str) -> std::result::Result<String, String> {
        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| format!("Failed to run osascript: {e}"))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

const APPLESCRIPT_REQUEST: &str = r#"tell application "iTerm2" to request cookie and key"#;

pub fn resolve_credentials(runner: &dyn AppleScriptRunner) -> Result<Credentials> {
    resolve_credentials_with_env(runner, |k| env::var(k).ok())
}

fn resolve_credentials_with_env(
    runner: &dyn AppleScriptRunner,
    env_fn: impl Fn(&str) -> Option<String>,
) -> Result<Credentials> {
    // Try env vars first
    if let (Some(cookie), Some(key)) = (env_fn("ITERM2_COOKIE"), env_fn("ITERM2_KEY")) {
        if !cookie.is_empty() && !key.is_empty() {
            return Ok(Credentials { cookie, key });
        }
    }

    // Fall back to osascript
    let output = runner
        .run_osascript(APPLESCRIPT_REQUEST)
        .map_err(|e| Error::Auth(format!("osascript failed: {e}")))?;

    parse_cookie_key(&output)
}

fn parse_cookie_key(output: &str) -> Result<Credentials> {
    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() == 2 {
        Ok(Credentials {
            cookie: parts[0].to_string(),
            key: parts[1].to_string(),
        })
    } else {
        Err(Error::Auth(
            "Failed to parse cookie/key from osascript output (expected two whitespace-separated tokens)".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRunner {
        result: std::result::Result<String, String>,
    }

    impl AppleScriptRunner for MockRunner {
        fn run_osascript(&self, _script: &str) -> std::result::Result<String, String> {
            self.result.clone()
        }
    }

    #[test]
    fn resolve_from_env_vars() {
        let runner = MockRunner {
            result: Err("should not be called".to_string()),
        };
        let creds = resolve_credentials_with_env(&runner, |key| match key {
            "ITERM2_COOKIE" => Some("test_cookie".to_string()),
            "ITERM2_KEY" => Some("test_key".to_string()),
            _ => None,
        })
        .unwrap();
        assert_eq!(creds.cookie, "test_cookie");
        assert_eq!(creds.key, "test_key");
    }

    #[test]
    fn empty_env_vars_fall_through_to_osascript() {
        let runner = MockRunner {
            result: Ok("abc123 def456".to_string()),
        };
        let creds = resolve_credentials_with_env(&runner, |key| match key {
            "ITERM2_COOKIE" => Some("".to_string()),
            "ITERM2_KEY" => Some("".to_string()),
            _ => None,
        })
        .unwrap();
        assert_eq!(creds.cookie, "abc123");
        assert_eq!(creds.key, "def456");
    }

    #[test]
    fn resolve_from_osascript() {
        let runner = MockRunner {
            result: Ok("abc123 def456".to_string()),
        };
        let creds =
            resolve_credentials_with_env(&runner, |_| None).unwrap();
        assert_eq!(creds.cookie, "abc123");
        assert_eq!(creds.key, "def456");
    }

    #[test]
    fn resolve_from_osascript_newline_separated() {
        let runner = MockRunner {
            result: Ok("abc123\ndef456".to_string()),
        };
        let creds =
            resolve_credentials_with_env(&runner, |_| None).unwrap();
        assert_eq!(creds.cookie, "abc123");
        assert_eq!(creds.key, "def456");
    }

    #[test]
    fn osascript_failure() {
        let runner = MockRunner {
            result: Err("connection refused".to_string()),
        };
        let err =
            resolve_credentials_with_env(&runner, |_| None).unwrap_err();
        assert!(err.to_string().contains("osascript failed"));
    }

    #[test]
    fn parse_bad_output() {
        let runner = MockRunner {
            result: Ok("justonetoken".to_string()),
        };
        let err =
            resolve_credentials_with_env(&runner, |_| None).unwrap_err();
        assert!(err.to_string().contains("Failed to parse"));
    }

    #[test]
    fn debug_redacts_credentials() {
        let creds = Credentials {
            cookie: "secret_cookie".to_string(),
            key: "secret_key".to_string(),
        };
        let debug_output = format!("{:?}", creds);
        assert!(!debug_output.contains("secret_cookie"));
        assert!(!debug_output.contains("secret_key"));
        assert!(debug_output.contains("[redacted]"));
    }
}
