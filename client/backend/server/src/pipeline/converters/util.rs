use crate::pipeline::error::ConverterError;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub async fn run_cmd(
    mut cmd: Command,
    name: &'static str,
    limit: Duration,
) -> Result<Vec<u8>, ConverterError> {
    cmd.kill_on_drop(true);

    let out = match timeout(limit, cmd.output()).await {
        Ok(r) => r?,
        Err(_) => {
            return Err(ConverterError::CommandFailed {
                cmd: name.into(),
                stderr: format!("timed out after {}s", limit.as_secs()),
            });
        }
    };

    if !out.status.success() {
        return Err(ConverterError::CommandFailed {
            cmd: name.into(),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }

    Ok(out.stdout)
}
