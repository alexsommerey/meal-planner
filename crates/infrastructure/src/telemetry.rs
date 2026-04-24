//! OpenTelemetry bootstrap: traces + metrics via OTLP/gRPC.

use anyhow::Context;
use futures_util::future::BoxFuture;
use opentelemetry::{KeyValue, Value, global, trace::TracerProvider as _};
use opentelemetry_otlp::{MetricExporter, SpanExporter as OtlpSpanExporter};
use opentelemetry_sdk::{
    Resource,
    export::trace::{ExportResult, SpanData, SpanExporter},
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime,
    trace::TracerProvider,
};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, prelude::*};

/// Shuts down providers on drop so buffered spans/metrics flush before exit.
pub struct Guard {
    tracer_provider: TracerProvider,
    meter_provider: SdkMeterProvider,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.tracer_provider.shutdown();
        let _ = self.meter_provider.shutdown();
    }
}

/// Reads `OTEL_*` env vars; defaults to `http://localhost:4317` (OTLP/gRPC).
pub fn init(service_name: &'static str) -> anyhow::Result<Guard> {
    let resource = Resource::new([KeyValue::new(SERVICE_NAME, service_name)]);

    let span_exporter = CodePathNormalizer::new(
        OtlpSpanExporter::builder()
            .with_tonic()
            .build()
            .context("build OTLP span exporter")?,
    );
    let tracer_provider = TracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(span_exporter, runtime::Tokio)
        .build();
    global::set_tracer_provider(tracer_provider.clone());

    let metric_exporter = MetricExporter::builder()
        .with_tonic()
        .build()
        .context("build OTLP metric exporter")?;
    let reader = PeriodicReader::builder(metric_exporter, runtime::Tokio).build();
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();
    global::set_meter_provider(meter_provider.clone());

    let tracer = tracer_provider.tracer(service_name);

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    Ok(Guard {
        tracer_provider,
        meter_provider,
    })
}

/// `SpanExporter` wrapper that rewrites `code.filepath` attributes before
/// export, so spans from third-party crates group across developers and
/// platforms instead of carrying machine-specific registry paths.
#[derive(Debug)]
struct CodePathNormalizer<E> {
    inner: E,
}

impl<E> CodePathNormalizer<E> {
    fn new(inner: E) -> Self {
        Self { inner }
    }
}

impl<E: SpanExporter> SpanExporter for CodePathNormalizer<E> {
    fn export(&mut self, mut batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        for span in &mut batch {
            for attr in &mut span.attributes {
                if attr.key.as_str() != "code.filepath" {
                    continue;
                }
                let rewritten = match &attr.value {
                    Value::String(s) => {
                        let current = s.as_str();
                        let new = normalize_registry_path(current);
                        (new != current).then_some(new)
                    }
                    _ => None,
                };
                if let Some(new) = rewritten {
                    attr.value = Value::String(new.into());
                }
            }
        }
        self.inner.export(batch)
    }

    fn shutdown(&mut self) {
        self.inner.shutdown();
    }

    fn force_flush(&mut self) -> BoxFuture<'static, ExportResult> {
        self.inner.force_flush()
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.inner.set_resource(resource);
    }
}

/// `<home>/.cargo/registry/src/<hash>/<crate>/<file>` → `registry:<crate>/<file>`.
/// Paths that don't match the registry pattern keep their content but have
/// backslashes normalized so Windows and Unix traces agree.
fn normalize_registry_path(path: &str) -> String {
    let unix = path.replace('\\', "/");
    const MARKER: &str = ".cargo/registry/src/";
    if let Some(start) = unix.find(MARKER) {
        let after_marker = &unix[start + MARKER.len()..];
        if let Some(after_hash) = after_marker.find('/') {
            return format!("registry:{}", &after_marker[after_hash + 1..]);
        }
    }
    unix
}

#[cfg(test)]
mod tests {
    use super::normalize_registry_path;

    #[test]
    fn rewrites_windows_cargo_registry() {
        let input = r"C:\Users\alex_\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\tower-http-0.6.8\src\trace\make_span.rs";
        assert_eq!(
            normalize_registry_path(input),
            "registry:tower-http-0.6.8/src/trace/make_span.rs",
        );
    }

    #[test]
    fn rewrites_unix_cargo_registry() {
        let input = "/home/bob/.cargo/registry/src/index.crates.io-deadbeef/tower-http-0.6.8/src/trace/make_span.rs";
        assert_eq!(
            normalize_registry_path(input),
            "registry:tower-http-0.6.8/src/trace/make_span.rs",
        );
    }

    #[test]
    fn normalizes_separators_on_workspace_paths() {
        let input = r"crates\infrastructure\src\telemetry.rs";
        assert_eq!(
            normalize_registry_path(input),
            "crates/infrastructure/src/telemetry.rs",
        );
    }
}
