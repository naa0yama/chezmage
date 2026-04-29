//! `OTel` provider initialization, shutdown, and process metrics.
//!
//! Gated by the `otel` feature; declared via `#[cfg(feature = "otel")] mod telemetry`
//! in `main.rs`.

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter};
use opentelemetry_sdk::{
    Resource, logs::SdkLoggerProvider, metrics::SdkMeterProvider,
    propagation::TraceContextPropagator, trace::SdkTracerProvider,
};
use tracing_subscriber::{
    filter::EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

/// Handles for all three `OTel` providers; each slot is `None` when
/// `OTEL_EXPORTER_OTLP_ENDPOINT` is unset or initialization fails.
pub type OtelProviders = (
    Option<SdkTracerProvider>,
    Option<SdkMeterProvider>,
    Option<SdkLoggerProvider>,
);

// SECURITY: Do not add process.command_args or process.environment
// resource detectors — they may expose identity file paths or the
// CHEZMOI_AGE_KEY environment variable to the OTel collector.
fn build_resource() -> Resource {
    use opentelemetry::KeyValue;
    use opentelemetry_semantic_conventions::attribute;

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| String::from(env!("CARGO_PKG_NAME")));
    Resource::builder()
        .with_service_name(service_name)
        .with_attributes([
            KeyValue::new(attribute::SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(
                attribute::SERVICE_INSTANCE_ID,
                gethostname::gethostname().to_string_lossy().into_owned(),
            ),
            KeyValue::new(attribute::VCS_REF_HEAD_REVISION, env!("GIT_HASH")),
        ])
        .build()
}

/// Initialize tracing subscriber with all three `OTel` providers.
///
/// Activates export only when `OTEL_EXPORTER_OTLP_ENDPOINT` is set and non-empty.
/// Returns provider handles for shutdown via [`shutdown_otel`].
///
/// Layer order: `EnvFilter` → `fmt` → `OTel` trace → `OTel` log bridge.
/// Log bridge is placed after trace layer so `LogRecord`s carry active span context.
// NOTEST(infra): tracing subscriber can only be initialized once per process
pub fn init_tracing() -> OtelProviders {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer();

    let (providers, trace_layer, log_layer) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|ep| !ep.is_empty())
        .and_then(|_| {
            let resource = build_resource();

            let span_exporter = SpanExporter::builder().with_http().build().ok()?;
            let tracer_provider = SdkTracerProvider::builder()
                .with_resource(resource.clone())
                .with_batch_exporter(span_exporter)
                .build();

            let log_exporter = LogExporter::builder().with_http().build().ok()?;
            let logger_provider = SdkLoggerProvider::builder()
                .with_resource(resource.clone())
                .with_batch_exporter(log_exporter)
                .build();

            let metric_exporter = MetricExporter::builder().with_http().build().ok()?;
            let meter_provider = SdkMeterProvider::builder()
                .with_periodic_exporter(metric_exporter)
                .with_resource(resource)
                .build();

            global::set_text_map_propagator(TraceContextPropagator::new());
            let tracer = tracer_provider.tracer(env!("CARGO_PKG_NAME"));
            global::set_tracer_provider(tracer_provider.clone());
            global::set_meter_provider(meter_provider.clone());

            // Build layers before logger_provider moves into the tuple.
            let log_layer = OpenTelemetryTracingBridge::new(&logger_provider);
            let trace_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            Some((
                (
                    Some(tracer_provider),
                    Some(meter_provider),
                    Some(logger_provider),
                ),
                Some(trace_layer),
                Some(log_layer),
            ))
        })
        .unwrap_or(((None, None, None), None, None));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(trace_layer)
        .with(log_layer)
        .init();

    providers
}

/// Flush and shut down all `OTel` providers in the correct order.
///
/// Order: tracer → meter (`force_flush` then `shutdown`) → logger last.
/// Logger shuts down last so tracer and meter errors still reach the log backend.
// NOTEST(infra): OTel provider shutdown
pub fn shutdown_otel((tracer, meter, logger): OtelProviders) {
    if let Some(p) = tracer
        && let Err(e) = p.shutdown()
    {
        tracing::warn!("OTel tracer shutdown failed: {e}");
    }
    if let Some(p) = meter {
        if let Err(e) = p.force_flush() {
            tracing::warn!("OTel meter flush failed: {e}");
        }
        if let Err(e) = p.shutdown() {
            tracing::warn!("OTel meter shutdown failed: {e}");
        }
    }
    if let Some(p) = logger
        && let Err(e) = p.shutdown()
    {
        tracing::warn!("OTel logger shutdown failed: {e}");
    }
}

#[cfg(all(feature = "process-metrics", not(miri)))]
pub use process_metrics::ProcessMetricHandles;

#[cfg(all(feature = "process-metrics", not(miri)))]
mod process_metrics {
    use std::sync::{Arc, Mutex};

    use opentelemetry::KeyValue;
    use opentelemetry::metrics::{
        Meter, ObservableCounter, ObservableGauge, ObservableUpDownCounter,
    };
    use opentelemetry_semantic_conventions::{attribute, metric as semconv};
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

    /// Observable instrument handles for the 8 standard process metrics.
    ///
    /// Drop to de-register all instruments. Keep alive for the program lifetime.
    #[derive(Debug)]
    pub struct ProcessMetricHandles {
        _cpu_time: ObservableCounter<f64>,
        _cpu_utilization: ObservableGauge<f64>,
        _mem_usage: ObservableUpDownCounter<i64>,
        _mem_virtual: ObservableUpDownCounter<i64>,
        _disk_io: ObservableCounter<u64>,
        _thread_count: ObservableUpDownCounter<i64>,
        _open_fds: ObservableUpDownCounter<i64>,
        _uptime: ObservableGauge<f64>,
    }

    impl ProcessMetricHandles {
        /// Register all 8 standard process observable instruments on `meter`.
        pub fn register(meter: &Meter) -> Self {
            #[allow(clippy::as_conversions)] // u32 → usize is a safe widening cast
            let pid = Pid::from(std::process::id() as usize);
            let cpu_count = std::thread::available_parallelism()
                .map(|n| u32::try_from(n.get()).unwrap_or(1))
                .unwrap_or(1);
            let system = Arc::new(Mutex::new(System::new_with_specifics(
                RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
            )));

            Self {
                _cpu_time: register_cpu_time(meter, Arc::clone(&system), pid),
                _cpu_utilization: register_cpu_utilization(
                    meter,
                    Arc::clone(&system),
                    pid,
                    cpu_count,
                ),
                _mem_usage: register_mem_usage(meter, Arc::clone(&system), pid),
                _mem_virtual: register_mem_virtual(meter, Arc::clone(&system), pid),
                _disk_io: register_disk_io(meter, Arc::clone(&system), pid),
                _thread_count: register_thread_count(meter, Arc::clone(&system), pid),
                _open_fds: register_open_fds(meter, Arc::clone(&system), pid),
                _uptime: register_uptime(meter, Arc::clone(&system), pid),
            }
        }
    }

    /// Refresh the target process and call `f` with its snapshot.
    ///
    /// Uses `try_lock` so a busy collection thread is skipped rather than blocked.
    fn with_process<F, T>(system: &Arc<Mutex<System>>, pid: Pid, f: F) -> Option<T>
    where
        F: FnOnce(&sysinfo::Process) -> T,
    {
        let mut sys = system.try_lock().ok()?;
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            false,
            ProcessRefreshKind::everything(),
        );
        sys.process(pid).map(f)
    }

    fn register_cpu_time(
        meter: &Meter,
        system: Arc<Mutex<System>>,
        pid: Pid,
    ) -> ObservableCounter<f64> {
        meter
            .f64_observable_counter(semconv::PROCESS_CPU_TIME)
            .with_unit("s")
            .with_description("Accumulated CPU time (user+system combined)")
            .with_callback(move |obs| {
                if let Some(ms) = with_process(&system, pid, sysinfo::Process::accumulated_cpu_time)
                {
                    #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
                    obs.observe(ms as f64 / 1000.0, &[]);
                }
            })
            .build()
    }

    fn register_cpu_utilization(
        meter: &Meter,
        system: Arc<Mutex<System>>,
        pid: Pid,
        cpu_count: u32,
    ) -> ObservableGauge<f64> {
        meter
            .f64_observable_gauge(semconv::PROCESS_CPU_UTILIZATION)
            .with_unit("1")
            .with_description("CPU utilization fraction 0..1 normalized by logical CPU count")
            .with_callback(move |obs| {
                if let Some(pct) = with_process(&system, pid, sysinfo::Process::cpu_usage) {
                    obs.observe(f64::from(pct) / (f64::from(cpu_count) * 100.0), &[]);
                }
            })
            .build()
    }

    fn register_mem_usage(
        meter: &Meter,
        system: Arc<Mutex<System>>,
        pid: Pid,
    ) -> ObservableUpDownCounter<i64> {
        meter
            .i64_observable_up_down_counter(semconv::PROCESS_MEMORY_USAGE)
            .with_unit("By")
            .with_description("Resident set size (RSS)")
            .with_callback(move |obs| {
                if let Some(bytes) = with_process(&system, pid, sysinfo::Process::memory)
                    && let Ok(v) = i64::try_from(bytes)
                {
                    obs.observe(v, &[]);
                }
            })
            .build()
    }

    fn register_mem_virtual(
        meter: &Meter,
        system: Arc<Mutex<System>>,
        pid: Pid,
    ) -> ObservableUpDownCounter<i64> {
        meter
            .i64_observable_up_down_counter(semconv::PROCESS_MEMORY_VIRTUAL)
            .with_unit("By")
            .with_description("Virtual memory size (VMS)")
            .with_callback(move |obs| {
                if let Some(bytes) = with_process(&system, pid, sysinfo::Process::virtual_memory)
                    && let Ok(v) = i64::try_from(bytes)
                {
                    obs.observe(v, &[]);
                }
            })
            .build()
    }

    fn register_disk_io(
        meter: &Meter,
        system: Arc<Mutex<System>>,
        pid: Pid,
    ) -> ObservableCounter<u64> {
        meter
            .u64_observable_counter(semconv::PROCESS_DISK_IO)
            .with_unit("By")
            .with_description("Total bytes of disk I/O split by direction")
            .with_callback(move |obs| {
                if let Some(usage) = with_process(&system, pid, sysinfo::Process::disk_usage) {
                    obs.observe(
                        usage.total_read_bytes,
                        &[KeyValue::new(attribute::DISK_IO_DIRECTION, "read")],
                    );
                    obs.observe(
                        usage.total_written_bytes,
                        &[KeyValue::new(attribute::DISK_IO_DIRECTION, "write")],
                    );
                }
            })
            .build()
    }

    fn register_thread_count(
        meter: &Meter,
        system: Arc<Mutex<System>>,
        pid: Pid,
    ) -> ObservableUpDownCounter<i64> {
        meter
            .i64_observable_up_down_counter(semconv::PROCESS_THREAD_COUNT)
            .with_unit("{thread}")
            .with_description("Number of threads in the process")
            .with_callback(move |obs| {
                let count = with_process(&system, pid, |p| {
                    #[cfg(target_os = "linux")]
                    {
                        p.tasks().map_or(1, std::collections::HashSet::len)
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        let _ = p;
                        1_usize
                    }
                })
                .unwrap_or(1);
                if let Ok(v) = i64::try_from(count) {
                    obs.observe(v, &[]);
                }
            })
            .build()
    }

    fn register_open_fds(
        meter: &Meter,
        system: Arc<Mutex<System>>,
        pid: Pid,
    ) -> ObservableUpDownCounter<i64> {
        meter
            .i64_observable_up_down_counter(semconv::PROCESS_OPEN_FILE_DESCRIPTOR_COUNT)
            .with_unit("{count}")
            .with_description("Number of open file descriptors")
            .with_callback(move |obs| {
                // open_files() returns Option<usize>; None on unsupported platforms.
                if let Some(Some(n)) = with_process(&system, pid, sysinfo::Process::open_files)
                    && let Ok(v) = i64::try_from(n)
                {
                    obs.observe(v, &[]);
                }
            })
            .build()
    }

    fn register_uptime(
        meter: &Meter,
        system: Arc<Mutex<System>>,
        pid: Pid,
    ) -> ObservableGauge<f64> {
        meter
            .f64_observable_gauge(semconv::PROCESS_UPTIME)
            .with_unit("s")
            .with_description("Process uptime in seconds")
            .with_callback(move |obs| {
                if let Some(secs) = with_process(&system, pid, sysinfo::Process::run_time) {
                    #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
                    obs.observe(secs as f64, &[]);
                }
            })
            .build()
    }
}
