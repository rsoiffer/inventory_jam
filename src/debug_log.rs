use std::time::{SystemTime, UNIX_EPOCH};

// #region agent log
pub fn agent_log(hypothesis_id: &str, location: &str, message: &str, data: &str) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let line = format!(
        r#"{{"sessionId":"cd7944","hypothesisId":"{hypothesis_id}","location":"{location}","message":"{message}","data":{data},"timestamp":{timestamp}}}"#
    );
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!(
            "{}/debug-cd7944.log",
            env!("CARGO_MANIFEST_DIR")
        ))
    {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
    }
}
// #endregion
