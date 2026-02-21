//! Brust - Rust ボイラープレートプロジェクト

/// ライブラリモジュール群
pub mod libs;
use clap::Parser;
use tracing_subscriber::filter::EnvFilter;
#[cfg(not(feature = "otel"))]
use tracing_subscriber::fmt;
#[cfg(feature = "otel")]
use tracing_subscriber::layer::SubscriberExt;
#[cfg(feature = "otel")]
use tracing_subscriber::util::SubscriberInitExt;

use crate::libs::hello::{GreetingError, sayhello};

#[derive(Parser)]
#[command(about)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value = "Youre")]
    name: String,
    /// Gender for greeting (man, woman)
    #[arg(short, long)]
    gender: Option<String>,
    #[arg(long, short = 'V', help = "Print version")]
    version: bool,
}

const APP_VERSION: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " version ",
    env!("CARGO_PKG_VERSION"),
    " (rev:",
    env!("GIT_HASH"),
    ")\n",
);

fn main() {
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

    let args = Args::parse();
    if args.version {
        tracing::info!("{}", APP_VERSION);
        #[allow(clippy::exit)] // CLIアプリケーションでの正当な使用
        std::process::exit(0);
    }

    run(&args.name, args.gender.as_deref());
}

/// アプリケーションのメイン処理を実行
///
/// # Arguments
/// * `name` - 挨拶対象の名前
/// * `gender` - 性別オプション（None, Some("man"), Some("woman"), その他）
pub fn run(name: &str, gender: Option<&str>) {
    let greeting = match sayhello(name, gender) {
        Ok(msg) => msg,
        Err(GreetingError::InvalidGender(invalid_gender)) => {
            tracing::warn!(
                "Invalid gender '{}' specified, using default greeting",
                invalid_gender
            );
            format!("Hi, {name} (invalid gender: {invalid_gender})")
        }
        Err(GreetingError::UnknownGender) => {
            tracing::error!("Unexpected error in greeting generation, using default");
            format!("Hi, {name}")
        }
    };

    tracing::info!("{}, new world!!", greeting);
}

#[cfg(test)]
mod tests {
    use super::run;
    use tracing::subscriber::with_default;
    use tracing_mock::{expect, subscriber};

    #[test]
    fn test_run_with_default_name() {
        let (subscriber, handle) = subscriber::mock()
            .event(expect::event().with_fields(expect::msg("Hi, Youre, new world!!")))
            .only()
            .run_with_handle();

        with_default(subscriber, || {
            run("Youre", None);
        });

        handle.assert_finished();
    }

    #[test]
    fn test_run_with_custom_name() {
        let (subscriber, handle) = subscriber::mock()
            .event(expect::event().with_fields(expect::msg("Hi, Alice, new world!!")))
            .only()
            .run_with_handle();

        with_default(subscriber, || {
            run("Alice", None);
        });

        handle.assert_finished();
    }

    #[test]
    fn test_run_with_empty_name() {
        let (subscriber, handle) = subscriber::mock()
            .event(expect::event().with_fields(expect::msg("Hi, , new world!!")))
            .only()
            .run_with_handle();

        with_default(subscriber, || {
            run("", None);
        });

        handle.assert_finished();
    }

    #[test]
    fn test_run_with_japanese_name() {
        let (subscriber, handle) = subscriber::mock()
            .event(expect::event().with_fields(expect::msg("Hi, 世界, new world!!")))
            .only()
            .run_with_handle();

        with_default(subscriber, || {
            run("世界", None);
        });

        handle.assert_finished();
    }

    #[test]
    fn test_run_with_gender_man() {
        let (subscriber, handle) = subscriber::mock()
            .event(expect::event().with_fields(expect::msg("Hi, Mr. John, new world!!")))
            .only()
            .run_with_handle();

        with_default(subscriber, || {
            run("John", Some("man"));
        });

        handle.assert_finished();
    }

    #[test]
    fn test_run_with_gender_woman() {
        let (subscriber, handle) = subscriber::mock()
            .event(expect::event().with_fields(expect::msg("Hi, Ms. Alice, new world!!")))
            .only()
            .run_with_handle();

        with_default(subscriber, || {
            run("Alice", Some("woman"));
        });

        handle.assert_finished();
    }

    #[test]
    fn test_run_with_invalid_gender() {
        use tracing_mock::expect;

        let (subscriber, handle) = subscriber::mock()
            .event(
                expect::event()
                    .with_target(env!("CARGO_PKG_NAME"))
                    .at_level(tracing::Level::WARN),
            )
            .event(
                expect::event()
                    .with_fields(expect::msg("Hi, Bob (invalid gender: other), new world!!")),
            )
            .run_with_handle();

        with_default(subscriber, || {
            run("Bob", Some("other"));
        });

        handle.assert_finished();
    }
}
