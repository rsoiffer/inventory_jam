use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::str::FromStr;

/// Edgegap environment variable containing the internal game port.
pub const PORT_ENV: &str = "ARBITRIUM_PORT_GAMEPORT_INTERNAL";
/// Edgegap environment variable containing the public IP or host.
pub const PUBLIC_IP_ENV: &str = "ARBITRIUM_PUBLIC_IP";
/// Edgegap environment variable containing the public game port.
pub const PUBLIC_PORT_ENV: &str = "ARBITRIUM_PORT_GAMEPORT_EXTERNAL";
/// Edgegap environment variable containing the deployment request id.
pub const REQUEST_ID_ENV: &str = "ARBITRIUM_REQUEST_ID";
/// DNS suffix used for Edgegap request-id hostnames.
pub const FQDN_SUFFIX: &str = ".pr.edgegap.net";

/// Connection metadata provided to game servers running inside Edgegap.
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    /// Edgegap deployment request id.
    pub request_id: String,
    /// Public IP or host value advertised by Edgegap.
    pub public_ip: String,
    /// Public port clients should connect to.
    pub public_port: u16,
    /// Internal port the server should bind inside the container.
    pub internal_port: u16,
    /// Derived public Edgegap DNS name for this deployment.
    pub fqdn: String,
}

/// Reads the Edgegap deployment environment, if this process is running there.
pub fn deployment_info() -> Option<DeploymentInfo> {
    let internal_port = env_u16(PORT_ENV)?;
    let request_id = std::env::var(REQUEST_ID_ENV).ok()?;
    let public_ip = std::env::var(PUBLIC_IP_ENV).unwrap_or_else(|_| request_id.clone());
    let public_port = env_u16(PUBLIC_PORT_ENV).unwrap_or(internal_port);
    let fqdn = format!("{request_id}{FQDN_SUFFIX}");
    Some(DeploymentInfo {
        request_id,
        public_ip,
        public_port,
        internal_port,
        fqdn,
    })
}

/// Returns whether the process appears to be running in Edgegap.
pub fn is_edgegap_server() -> bool {
    std::env::var(PORT_ENV).is_ok()
}

/// Converts Edgegap public connection metadata into a socket address.
pub fn host_port_from_deployment(deployment: &DeploymentInfo) -> SocketAddr {
    if let Ok(ip) = IpAddr::from_str(&deployment.public_ip) {
        return SocketAddr::new(ip, deployment.public_port);
    }
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), deployment.public_port)
}

/// Reads and parses an environment variable as a `u16`.
fn env_u16(name: &str) -> Option<u16> {
    std::env::var(name).ok()?.parse().ok()
}
