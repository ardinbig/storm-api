use std::time::Duration;

use testcontainers::ContainerAsync;

/// Repeatedly asks Testcontainers for a mapped host port until Docker reports it.
///
/// This smooths over occasional timing gaps where the container is started but
/// the port mapping is not yet visible to the Docker inspection API.
pub async fn host_port_ipv4_with_retry<C>(container: &ContainerAsync<C>, port: u16) -> u16
where
    C: testcontainers::core::Image,
{
    const RETRIES: usize = 300;
    const DELAY: Duration = Duration::from_millis(100);

    for _ in 0..RETRIES {
        match container.get_host_port_ipv4(port).await {
            Ok(mapped) => return mapped,
            Err(_) => {
                tokio::time::sleep(DELAY).await;
            }
        }
    }

    panic!("failed to resolve mapped host port for container port {port} after {RETRIES} retries")
}
