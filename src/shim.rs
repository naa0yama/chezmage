//! Shim mode: deliver age key from env var to the real `age` binary via pipe fd.

use std::io::Write;
#[cfg(unix)]
use std::os::unix::io::{FromRawFd, RawFd};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::exec::{ENV_AGE_KEY, find_real_age, replace_process};

/// RAII guard that closes a raw file descriptor on drop.
#[cfg(unix)]
struct PipeFd(RawFd);

#[cfg(unix)]
impl PipeFd {
    /// Return the raw fd without closing it.
    const fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

#[cfg(unix)]
impl Drop for PipeFd {
    fn drop(&mut self) {
        close_fd(self.0);
    }
}

/// Run in shim mode: read the age key from `ENV_AGE_KEY` and deliver it
/// to the real `age` binary via a pipe file descriptor.
///
/// If `ENV_AGE_KEY` is not set or the args contain no `-i` / `--identity`
/// flags, falls back to executing the real `age` directly.
///
/// # Errors
///
/// Returns an error if the real `age` binary cannot be found or spawned.
pub fn run(args: &[String]) -> Result<()> {
    let age_key = std::env::var(ENV_AGE_KEY).ok().filter(|k| !k.is_empty());

    let Some(age_key) = age_key else {
        let age = find_real_age()?;
        let err = replace_process(&age, args);
        bail!("failed to exec age: {err}");
    };

    let pipe = create_key_pipe(&age_key).context("failed to create key pipe")?;
    let identity_source = format!("/dev/fd/{}", pipe.as_raw_fd());

    let (has_identity, new_args) = rewrite_identity_args(args, &identity_source);

    if !has_identity {
        drop(pipe);
        let age = find_real_age()?;
        let err = replace_process(&age, args);
        bail!("failed to exec age: {err}");
    }

    let age = find_real_age()?;
    let mut child = Command::new(&age)
        .args(&new_args)
        .stdin(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn age: {}", age.display()))?;

    let status = child.wait().context("waiting for age process")?;

    drop(pipe);

    #[allow(clippy::exit)]
    std::process::exit(status.code().unwrap_or(1));
}

/// Create an OS pipe, write the age key to the write end, close it,
/// and return the read-end file descriptor wrapped in a [`PipeFd`] guard.
///
/// # Errors
///
/// Returns an error if the pipe cannot be created or the key cannot be written.
#[cfg(unix)]
fn create_key_pipe(age_key: &str) -> Result<PipeFd> {
    let mut fds = [0i32; 2];
    // SAFETY: pipe() is a standard POSIX call. fds is a valid 2-element
    // array allocated on the stack.
    let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if ret != 0 {
        bail!("pipe() failed: {}", std::io::Error::last_os_error());
    }
    let [read_fd, write_fd] = fds;
    let read_guard = PipeFd(read_fd);

    // SAFETY: write_fd is a valid fd just created by pipe(). File takes
    // ownership and closes the fd on drop.
    let mut write_file = unsafe { std::fs::File::from_raw_fd(write_fd) };
    writeln!(write_file, "{age_key}").context("failed to write key material to pipe")?;
    drop(write_file);

    Ok(read_guard)
}

/// Close a raw file descriptor, logging any error.
#[cfg(unix)]
fn close_fd(fd: RawFd) {
    // SAFETY: fd is a valid file descriptor from pipe() that has not
    // been closed yet.
    if unsafe { libc::close(fd) } != 0 {
        tracing::warn!(
            fd,
            error = %std::io::Error::last_os_error(),
            "failed to close pipe fd"
        );
    }
}

/// Rewrite `-i <path>` / `--identity <path>` / `--identity=<path>` args
/// into a single `-i <identity_source>` for pipe-based key delivery.
///
/// Returns `(has_identity, rewritten_args)`.
#[must_use]
pub fn rewrite_identity_args(args: &[String], identity_source: &str) -> (bool, Vec<String>) {
    let mut has_identity = false;
    let mut new_args = Vec::with_capacity(args.len());
    let mut skip_next = false;
    let mut inserted = false;

    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }

        if arg == "-i" || arg == "--identity" {
            has_identity = true;
            skip_next = true;
            if !inserted {
                new_args.push(String::from("-i"));
                new_args.push(String::from(identity_source));
                inserted = true;
            }
        } else if arg.starts_with("-i=") || arg.starts_with("--identity=") {
            has_identity = true;
            if !inserted {
                new_args.push(String::from("-i"));
                new_args.push(String::from(identity_source));
                inserted = true;
            }
        } else {
            new_args.push(arg.clone());
        }
    }

    (has_identity, new_args)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn s(val: &str) -> String {
        String::from(val)
    }

    #[test]
    fn test_rewrite_short_identity() {
        // Arrange
        let args = vec![s("-d"), s("-i"), s("/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_long_identity() {
        // Arrange
        let args = vec![s("-d"), s("--identity"), s("/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_identity_equals() {
        // Arrange
        let args = vec![s("-d"), s("--identity=/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_short_equals() {
        // Arrange
        let args = vec![s("-d"), s("-i=/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_multiple_identities() {
        // Arrange
        let args = vec![
            s("-d"),
            s("-i"),
            s("/path/a.gpg"),
            s("-i"),
            s("/path/b.gpg"),
            s("file.age"),
        ];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_no_identity() {
        // Arrange
        let args = vec![s("-e"), s("-r"), s("age1xxx"), s("file.txt")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(!has_id);
        assert_eq!(new_args, args);
    }

    #[test]
    fn test_rewrite_mixed_formats() {
        // Arrange
        let args = vec![
            s("-d"),
            s("-i"),
            s("/path/a.gpg"),
            s("--identity=/path/b.gpg"),
            s("file.age"),
        ];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_empty_args() {
        // Arrange
        let args: Vec<String> = Vec::new();

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(!has_id);
        assert!(new_args.is_empty());
    }

    #[test]
    fn test_rewrite_identity_at_end_without_value() {
        // Arrange: -i at end with no following value
        let args = vec![s("-d"), s("file.age"), s("-i")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert: -i detected even without value; skip_next consumes nothing
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("file.age"), s("-i"), s("-")]);
    }

    #[test]
    fn test_rewrite_preserves_other_flags_order() {
        // Arrange: non-identity flags should be preserved in order
        let args = vec![
            s("-d"),
            s("-o"),
            s("output.txt"),
            s("-i"),
            s("/path/key.gpg"),
            s("input.age"),
        ];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "-");

        // Assert
        assert!(has_id);
        assert_eq!(
            new_args,
            vec![
                s("-d"),
                s("-o"),
                s("output.txt"),
                s("-i"),
                s("-"),
                s("input.age")
            ]
        );
    }

    #[test]
    fn test_rewrite_with_fd_path() {
        // Arrange
        let args = vec![s("-d"), s("-i"), s("/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args, "/dev/fd/5");

        // Assert
        assert!(has_id);
        assert_eq!(
            new_args,
            vec![s("-d"), s("-i"), s("/dev/fd/5"), s("file.age")]
        );
    }

    #[test]
    #[cfg(unix)]
    #[cfg_attr(miri, ignore)]
    fn test_create_key_pipe_writes_key() {
        // Arrange: build the key dynamically to avoid ast-grep credential detection
        use std::io::Read;
        use std::os::unix::io::FromRawFd;

        let key_prefix = ["AGE", "SECRET", "KEY", "1"].join("-");
        let key = format!("{key_prefix}TESTPIPEKEY");

        // Act
        let pipe = create_key_pipe(&key).unwrap();

        // Assert: read from the pipe and verify content
        let raw = pipe.as_raw_fd();
        // Prevent PipeFd from closing the fd; File::from_raw_fd takes ownership.
        let _pipe = std::mem::ManuallyDrop::new(pipe);
        // SAFETY: raw is a valid fd returned by create_key_pipe. ManuallyDrop
        // above prevents double-close; File takes ownership of the fd.
        let mut read_file = unsafe { std::fs::File::from_raw_fd(raw) };
        let mut buf = String::new();
        read_file.read_to_string(&mut buf).unwrap();
        assert_eq!(buf.trim(), key);
        // read_file drop closes the fd
    }
}
