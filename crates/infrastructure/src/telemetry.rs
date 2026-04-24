//! OpenTelemetry bootstrap: traces + metrics via OTLP/gRPC.

use anyhow::Context;
use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_otlp::{MetricExporter, SpanExporter};
use opentelemetry_sdk::{
    Resource,
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

    let span_exporter = SpanExporter::builder()
        .with_tonic()
        .build()
        .context("build OTLP span exporter")?;
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
