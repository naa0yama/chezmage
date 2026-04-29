//! chezmage — chezmoi + memory + age: GPG-backed age encryption wrapper.
//!
//! Single binary for protecting age secret keys with GPG (`YubiKey`).
//! Keys never touch disk — they exist only in process memory (env var).
//!
//! ## Modes
//!
//! - `chezmage <args>` — **Wrapper**: reads chezmoi.toml, decrypts GPG
//!   identities, sets env var, execs chezmoi
//! - `chezmage --shim <args>` — **Shim**: pipes env var key to the real
//!   `age` binary via stdin

/// Chezmoi configuration parsing.
pub mod config;
/// Process execution and path utilities.
pub mod exec;
/// GPG decryption.
pub mod gpg;
/// Security primitives (SecureString, process hardening).
pub mod secure;
/// Shim mode (age stdin pipe).
pub mod shim;
/// `OTel` provider initialization, shutdown, and process metrics.
#[cfg(feature = "otel")]
mod telemetry;
/// Wrapper mode (identity discovery + chezmoi exec).
pub mod wrapper;

#[cfg(test)]
pub(crate) mod test_utils;

use std::env;
use std::io::Write as _;

use clap::Parser;
#[cfg(not(feature = "otel"))]
use tracing_subscriber::filter::EnvFilter;

/// `OTel` provider handle for shutdown; `()` when the `otel` feature is disabled.
#[cfg(feature = "otel")]
type OtelHandle = telemetry::OtelProviders;
#[cfg(not(feature = "otel"))]
type OtelHandle = ();

/// Application version string including git revision.
const APP_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (rev:", env!("GIT_HASH"), ")",);

/// Application about string shown in `--help`.
const APP_ABOUT: &str = concat!(
    env!("CARGO_PKG_DESCRIPTION"),
    "\nversion ",
    env!("CARGO_PKG_VERSION"),
    " (rev:",
    env!("GIT_HASH"),
    ")",
);

/// chezmage CLI arguments.
#[derive(Parser, Debug)]
#[command(about = APP_ABOUT, version = APP_VERSION)]
struct Args {
    /// Run in shim mode (pipe age key via stdin).
    #[arg(long)]
    shim: bool,

    /// Arguments passed through to chezmoi.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

// NOTEST(infra): entry point — tested via integration tests (assert_cmd)
fn main() {
    secure::harden_process();

    let raw_args: Vec<String> = env::args().skip(1).collect();

    if let Some(pos) = raw_args.iter().position(|a| a == "--shim") {
        // Shim mode: do NOT init tracing — chezmoi captures stderr as
        // part of the decrypted plaintext, so any output contaminates it.
        let mut shim_args = raw_args;
        shim_args.remove(pos);
        if let Err(e) = shim::run(&shim_args) {
            let _ = writeln!(std::io::stderr(), "chezmage: shim error: {e:#}");
            #[allow(clippy::exit)]
            std::process::exit(1);
        }
    } else {
        // Wrapper mode: init tracing normally
        let otel_handle = init_tracing();
        let _args = Args::parse();

        let exit_code = {
            // Keep process metric handles alive until this block exits,
            // ensuring de-registration happens before meter provider shutdown.
            #[cfg(all(feature = "process-metrics", not(miri)))]
            let _pm = {
                let meter = opentelemetry::global::meter(env!("CARGO_PKG_NAME"));
                telemetry::ProcessMetricHandles::register(&meter)
            };

            // Root span wraps all command processing so child spans share a
            // single trace_id. Block scope ensures the span exits before shutdown.
            let root = tracing::info_span!("main");
            let _guard = root.enter();
            if let Err(e) = wrapper::run() {
                tracing::error!("{e:#}");
                1i32
            } else {
                0i32
            }
        }; // _pm drops here — before shutdown_tracing

        shutdown_tracing(otel_handle);

        if exit_code != 0 {
            #[allow(clippy::exit)]
            std::process::exit(exit_code);
        }
    }
}

/// Initialize tracing subscriber (fmt only, no `OTel`).
// NOTEST(infra): tracing subscriber can only be initialized once per process
#[cfg(not(feature = "otel"))]
fn init_tracing() -> OtelHandle {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}

/// Initialize tracing subscriber with all three `OTel` providers.
// NOTEST(infra): tracing subscriber can only be initialized once per process
#[cfg(feature = "otel")]
fn init_tracing() -> OtelHandle {
    telemetry::init_tracing()
}

/// Flush and shut down `OTel` providers before process exit.
// NOTEST(infra): OTel provider shutdown
#[cfg(feature = "otel")]
fn shutdown_tracing(handle: OtelHandle) {
    telemetry::shutdown_otel(handle);
}

/// No-op when the `otel` feature is disabled.
#[cfg(not(feature = "otel"))]
fn shutdown_tracing(_handle: OtelHandle) {}
