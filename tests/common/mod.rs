//! Common integration test utilities, shared mock server builders, and mutex-guarded ENV_LOCK.

use std::sync::Mutex;

/// Global environment variable lock to avoid race conditions across concurrent tests.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Guard struct that restores environment variables when dropped.
pub struct EnvGuard {
    vars: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    /// Sets a temporary environment variable and records the previous value for cleanup.
    #[allow(dead_code)]
    pub fn set(key: &str, val: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, val);
        Self {
            vars: vec![(key.to_string(), prev)],
        }
    }

    /// Sets multiple environment variables.
    pub fn set_many(vars: &[(&str, &str)]) -> Self {
        let mut recorded = Vec::new();
        for (k, v) in vars {
            recorded.push((k.to_string(), std::env::var(k).ok()));
            std::env::set_var(k, v);
        }
        Self { vars: recorded }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, prev) in &self.vars {
            match prev {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}
