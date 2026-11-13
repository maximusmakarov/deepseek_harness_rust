use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static PROVIDER: std::sync::OnceLock<SdkTracerProvider> = std::sync::OnceLock::new();

/// Initializes JSON logging and, when OTLP is configured, a tracing layer.
pub fn init(level: Option<&str>) {
    let requested = std::env::var("RUST_LOG")
        .ok()
        .or_else(|| level.map(ToOwned::to_owned))
        .unwrap_or_else(|| "info".to_owned());
    let filter = tracing_subscriber::EnvFilter::try_new(requested)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(std::io::stderr);

    if otlp_enabled() {
        match exporter() {
            Ok(exporter) => {
                let provider = SdkTracerProvider::builder()
                    .with_batch_exporter(exporter)
                    .with_resource(
                        Resource::builder()
                            .with_service_name(service_name())
                            .build(),
                    )
                    .build();
                let tracer = provider.tracer("dshr");
                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                let _ = PROVIDER.set(provider.clone());
                opentelemetry::global::set_tracer_provider(provider);
                let _ = tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .with(otel_layer)
                    .try_init();
            }
            Err(error) => {
                eprintln!("otel disabled: {error}");
                let _ = tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .try_init();
            }
        }
    } else {
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .try_init();
    }
}

/// Flushes and shuts down the OTLP provider, if configured.
pub fn shutdown() {
    if let Some(provider) = PROVIDER.get() {
        let _ = provider.shutdown();
    }
}

fn otlp_enabled() -> bool {
    std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_some()
        || std::env::var_os("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_some()
}

fn exporter() -> Result<SpanExporter, String> {
    SpanExporter::builder()
        .with_http()
        .build()
        .map_err(|error| error.to_string())
}

fn service_name() -> String {
    std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "dshr".to_owned())
}
