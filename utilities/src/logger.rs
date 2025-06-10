use tracing::level_filters::LevelFilter;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

// exporing the info! warn! etc tracing macro through this Library
pub use tracing::{debug, error, info, trace, warn};

pub fn init_logger(service_name: &str, node_id: &str) -> WorkerGuard {
    let file_appender = RollingFileAppender::new(
        Rotation::NEVER,
        format!(
            "/Users/lovepreetsingh/Library/Logs/whispiring_woods/{}",
            service_name
        ),
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
        .flatten_event(true);
    let stdout_layer = fmt::layer().with_writer(std::io::stdout);
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::registry()
        .with(json_layer)
        .with(stdout_layer)
        .with(filter)
        .init();
    info!(service = %service_name,node_id = %node_id,"Logging initialized");
    _gaurd
}
