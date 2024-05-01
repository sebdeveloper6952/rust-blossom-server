use crate::telemetry::stdout::init_stdout_tracing;
use crate::telemetry::uptrace::init_uptrace_tracing;

pub fn init_tracing(dsn: String, env: String, service_name: String) -> Result<(), String> {
    if env == "LOCAL" {
        return init_stdout_tracing(service_name);
    }

    return init_uptrace_tracing(dsn, env, service_name);
}
