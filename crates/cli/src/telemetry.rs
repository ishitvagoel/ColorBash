use std::time::Instant;

pub fn trace_duration(event: &str, started: Instant) {
    if tracing_enabled() {
        eprintln!(
            "mbx trace event={event} elapsed_us={}",
            started.elapsed().as_micros()
        );
    }
}

pub fn trace_message(message: &str) {
    if tracing_enabled() {
        eprintln!("mbx trace {message}");
    }
}

fn tracing_enabled() -> bool {
    std::env::var("MBX_LOG").is_ok_and(|value| value == "trace")
}
