use std::time;

use super::safe_child::SafeChild;

pub(crate) async fn get_acquired_port(
    process: &mut SafeChild,
    sleep_time: time::Duration,
    max_retries: usize,
) -> Result<u16, anyhow::Error> {
    let pid = process.id();
    for _ in 0..max_retries {
        if let Ok(ports) = listeners::get_ports_by_pid(pid) {
            if ports.len() == 1 {
                return Ok(ports.into_iter().next().unwrap());
            }
        }

        if let Ok(Some(status)) = process.process.try_wait() {
            return Err(anyhow::anyhow!("Background Devnet process exited with status {status}"));
        }

        tokio::time::sleep(sleep_time).await;
    }

    Err(anyhow::anyhow!("Could not identify a unique port used by PID {pid}"))
}
