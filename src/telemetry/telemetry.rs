use crate::config::TelemetryKind;
use crate::telemetry::stdout::init_stdout_tracing;
use crate::telemetry::uptrace::init_uptrace_tracing;

pub fn init_tracing(
    dsn: String,
    service_name: String,
    env: String,
    kind: TelemetryKind,
) -> Result<(), String> {
    match kind {
        TelemetryKind::Stdout => init_stdout_tracing(service_name),
        TelemetryKind::Uptrace => init_uptrace_tracing(dsn, env, service_name),
        _ => Ok(()),
    }
}
