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
/// Wrapper mode (identity discovery + chezmoi exec).
pub mod wrapper;

#[cfg(test)]
pub(crate) mod test_utils;

use std::env;

use clap::Parser;
use tracing_subscriber::filter::EnvFilter;
#[cfg(not(feature = "otel"))]
use tracing_subscriber::fmt;
#[cfg(feature = "otel")]
use tracing_subscriber::layer::SubscriberExt;
#[cfg(feature = "otel")]
use tracing_subscriber::util::SubscriberInitExt;

/// Application version string including git revision.
const APP_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (rev:", env!("GIT_HASH"), ")",);

/// chezmage CLI arguments.
#[derive(Parser, Debug)]
#[command(about, version = APP_VERSION)]
struct Args {
    /// Arguments passed through to chezmoi.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() {
    secure::harden_process();

    init_tracing();

    let raw_args: Vec<String> = env::args().skip(1).collect();

    if let Some(pos) = raw_args.iter().position(|a| a == "--shim") {
        // Shim mode: strip --shim, pass remaining args to age pipe
        let mut shim_args = raw_args;
        shim_args.remove(pos);
        if let Err(e) = shim::run(&shim_args) {
            tracing::error!("{e:#}");
            #[allow(clippy::exit)]
            std::process::exit(1);
        }
    } else {
        // Wrapper mode: handle --help/--version via clap, then exec chezmoi
        let _args = Args::parse();

        if let Err(e) = wrapper::run() {
            tracing::error!("{e:#}");
            #[allow(clippy::exit)]
            std::process::exit(1);
        }
    }
}

/// Initialize tracing subscriber with optional OpenTelemetry layer.
fn init_tracing() {
    #[cfg(not(feature = "otel"))]
    {
        fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .init();
    }

    #[cfg(feature = "otel")]
    {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let fmt_layer = tracing_subscriber::fmt::layer();

        let otel_layer = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .ok()
            .and_then(|_| {
                let exporter = opentelemetry_otlp::SpanExporter::builder()
                    .with_http()
                    .build()
                    .ok()?;

                let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                    .with_simple_exporter(exporter)
                    .build();

                let tracer = opentelemetry::trace::TracerProvider::tracer(
                    &tracer_provider,
                    env!("CARGO_PKG_NAME"),
                );
                opentelemetry::global::set_tracer_provider(tracer_provider);

                Some(tracing_opentelemetry::layer().with_tracer(tracer))
            });

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer)
            .init();
    }
}
