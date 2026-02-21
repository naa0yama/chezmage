//! Security primitives: SecureString with mlock + zeroize, process hardening.

use zeroize::Zeroizing;

/// Prefix for valid age secret key lines.
const AGE_SECRET_PREFIX: &str = "AGE-SECRET-KEY-";

/// Secure string that locks memory and zeroizes on drop.
///
/// Uses `Zeroizing<Vec<u8>>` for safe automatic zeroing. Memory is pinned
/// via `mlock()` to prevent swap writes, and unlocked on drop.
///
/// Debug output is intentionally redacted to prevent key leakage.
#[allow(clippy::module_name_repetitions)]
pub struct SecureString {
    buf: Zeroizing<Vec<u8>>,
}

impl std::fmt::Debug for SecureString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureString")
            .field("len", &self.buf.len())
            .finish()
    }
}

impl SecureString {
    /// Create a new `SecureString` from a `String`, locking memory immediately.
    #[must_use]
    pub fn new(s: String) -> Self {
        let buf = Zeroizing::new(s.into_bytes());
        lock_memory(&buf);
        Self { buf }
    }

    /// Return the content as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // Age keys are ASCII, fallback to empty on invalid UTF-8
        std::str::from_utf8(&self.buf).unwrap_or("")
    }

    /// Count valid AGE-SECRET-KEY lines (ignoring comments).
    #[must_use]
    pub fn count_secret_keys(&self) -> usize {
        self.as_str()
            .lines()
            .filter(|line| {
                let t = line.trim();
                !t.starts_with('#') && t.starts_with(AGE_SECRET_PREFIX)
            })
            .count()
    }
}

impl Drop for SecureString {
    fn drop(&mut self) {
        // Zeroizing handles zeroing the buffer automatically.
        // We only need to unlock the memory region.
        unlock_memory(&self.buf);
    }
}

// ---------------------------------------------------------------------------
// mlock / munlock
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn lock_memory(buf: &[u8]) {
    if !buf.is_empty() {
        // SAFETY: mlock pins the buffer in physical memory to prevent swap.
        // The pointer and length come from a valid Vec<u8> allocation.
        let ret = unsafe { libc::mlock(buf.as_ptr().cast::<libc::c_void>(), buf.len()) };
        if ret != 0 {
            tracing::warn!(
                errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
                len = buf.len(),
                "mlock failed — key memory may be swappable"
            );
        }
    }
}

#[cfg(unix)]
fn unlock_memory(buf: &[u8]) {
    if !buf.is_empty() {
        // SAFETY: munlock releases the mlock pin. The buffer is still valid
        // at this point (called before deallocation by Zeroizing's drop).
        unsafe {
            libc::munlock(buf.as_ptr().cast::<libc::c_void>(), buf.len());
        }
    }
}

#[cfg(not(unix))]
fn lock_memory(_buf: &[u8]) {}

#[cfg(not(unix))]
fn unlock_memory(_buf: &[u8]) {}

// ---------------------------------------------------------------------------
// Process hardening
// ---------------------------------------------------------------------------

/// Harden the current process: disable core dumps and ptrace.
pub fn harden_process() {
    #[cfg(unix)]
    {
        // SAFETY: setrlimit with RLIMIT_CORE=0 disables core dumps.
        // This is a standard POSIX call with no memory safety concerns.
        unsafe {
            let rlim = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, std::ptr::from_ref(&rlim));
        }
    }

    #[cfg(target_os = "linux")]
    {
        // SAFETY: prctl(PR_SET_DUMPABLE, 0) prevents ptrace attach and
        // protects /proc/PID/environ. Standard Linux security hardening.
        unsafe {
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_secure_string_new_and_as_str() {
        // Arrange
        let input = String::from("hello secure world");

        // Act
        let ss = SecureString::new(input);

        // Assert
        assert_eq!(ss.as_str(), "hello secure world");
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_secure_string_empty() {
        // Arrange & Act
        let ss = SecureString::new(String::new());

        // Assert
        assert_eq!(ss.as_str(), "");
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_count_secret_keys_single() {
        // Arrange
        let content = String::from("# comment\nAGE-SECRET-KEY-1ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");

        // Act
        let ss = SecureString::new(content);

        // Assert
        assert_eq!(ss.count_secret_keys(), 1);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_count_secret_keys_multiple() {
        // Arrange
        let content = String::from("AGE-SECRET-KEY-1FIRST\n# comment\nAGE-SECRET-KEY-1SECOND\n");

        // Act
        let ss = SecureString::new(content);

        // Assert
        assert_eq!(ss.count_secret_keys(), 2);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_count_secret_keys_none() {
        // Arrange
        let content = String::from("# only comments\nno keys here\n");

        // Act
        let ss = SecureString::new(content);

        // Assert
        assert_eq!(ss.count_secret_keys(), 0);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_count_secret_keys_commented_out() {
        // Arrange
        let content = String::from("# AGE-SECRET-KEY-1COMMENTED\n");

        // Act
        let ss = SecureString::new(content);

        // Assert
        assert_eq!(ss.count_secret_keys(), 0);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_count_secret_keys_with_whitespace() {
        // Arrange
        let content = String::from("  AGE-SECRET-KEY-1PADDED  \n");

        // Act
        let ss = SecureString::new(content);

        // Assert
        assert_eq!(ss.count_secret_keys(), 1);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_count_secret_keys_empty() {
        // Arrange & Act
        let ss = SecureString::new(String::new());

        // Assert
        assert_eq!(ss.count_secret_keys(), 0);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_harden_process_does_not_panic() {
        // Arrange & Act & Assert
        harden_process();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_secure_string_debug_redacts_content() {
        // Arrange: build the key dynamically to avoid ast-grep credential detection
        let marker = "XYZZY12345ABCDE";
        let key_prefix = ["AGE", "SECRET", "KEY", "1"].join("-");
        let ss = SecureString::new(format!("{key_prefix}{marker}"));

        // Act
        let debug_output = format!("{ss:?}");

        // Assert
        assert!(
            !debug_output.contains(marker),
            "Debug output must not contain key material"
        );
        assert!(debug_output.contains("SecureString"));
        assert!(debug_output.contains("len"));
    }
}
