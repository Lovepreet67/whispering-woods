use opentelemetry::{KeyValue, runtime::Tokio};
use opentelemetry_otlp::{WithExportConfig, new_exporter, new_pipeline};
use opentelemetry_sdk::{Resource, trace::Tracer};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
// exporing the info! warn! etc tracing macro through this Library
pub use tracing;
pub use tracing::*;

use crate::result::Result;

pub fn init_apm(service_name: &str, node_id: &str, endpoint: &str) -> Result<Tracer> {
    let otlp_exporter = new_exporter().http().with_endpoint(endpoint);
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.node.id", node_id.to_string()),
        KeyValue::new("service.version", "1.0.0"),
        KeyValue::new("deployment.environment", "production"),
    ]);
    let tracer = new_pipeline()
        .tracing()
        .with_trace_config(opentelemetry_sdk::trace::config().with_resource(resource))
        .with_exporter(otlp_exporter)
        .install_batch(Tokio)
        .unwrap();
    Ok(tracer)
}
pub fn init_logger(service_name: &str, node_id: &str, level: String) -> WorkerGuard {
    let env = std::env::var("ENV").unwrap_or("local".to_owned());
    let log_base = match &env[..] {
        "local" => "/Users/lovepreetsingh/Library/Logs/whispiring_woods",
        _ => "logs",
    };
    let file_appender = RollingFileAppender::new(
        Rotation::NEVER,
        format!("{log_base}/{}", service_name),
        format!("{}.log", node_id),
    );
    let (non_blocking, _gaurd) = tracing_appender::non_blocking(file_appender);
    let json_layer = fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_line_number(true)
        .with_file(true)
        .with_thread_names(true)
        .with_current_span(true)
        .with_target(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
        .flatten_event(true);
    let stdout_layer = fmt::layer().with_writer(std::io::stdout);
    let filter = EnvFilter::builder()
        .with_default_directive(level.parse::<Level>().unwrap_or(Level::INFO).into())
        .from_env_lossy();

    // code related to telemetry
    let apm_endpoint = std::env::var("APM_ENDPOINT").unwrap_or("http://locahost:8200".to_owned());
    // for opentelemetry export
    let tracer = match init_apm(service_name, node_id, &apm_endpoint) {
        Ok(v) => v,
        Err(e) => {
            error!("Error while creating tracer endpoint:{apm_endpoint}, error:{e:?}");
            panic!("Error while creating tracer for metric export");
        }
    };
    let telemetery_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(json_layer)
        .with(stdout_layer)
        .with(filter)
        .with(telemetery_layer)
        .init();
    info!(service = %service_name,node_id = %node_id,"Logging initialized");
    info!(%apm_endpoint,"Got apm endpoint");
    _gaurd
}
