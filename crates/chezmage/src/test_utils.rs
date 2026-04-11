//! Shared test utilities (only compiled when `#[cfg(test)]`).

use std::env;
use std::sync::Mutex;

/// Mutex to serialize tests that mutate process-wide environment variables.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// RAII guard that saves and restores an environment variable on drop.
pub struct EnvGuard {
    key: String,
    old: Option<String>,
}

impl EnvGuard {
    /// Set `key` to `val`, saving the previous value for restoration.
    pub fn set(key: &str, val: &str) -> Self {
        let old = env::var(key).ok();
        // SAFETY: Tests run single-threaded (serialized by ENV_LOCK).
        unsafe { env::set_var(key, val) };
        Self {
            key: String::from(key),
            old,
        }
    }

    /// Remove `key`, saving the previous value for restoration.
    pub fn remove(key: &str) -> Self {
        let old = env::var(key).ok();
        // SAFETY: Tests run single-threaded (serialized by ENV_LOCK).
        unsafe { env::remove_var(key) };
        Self {
            key: String::from(key),
            old,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: Tests run single-threaded (serialized by ENV_LOCK).
        unsafe {
            if let Some(ref val) = self.old {
                env::set_var(&self.key, val);
            } else {
                env::remove_var(&self.key);
            }
        }
    }
}
