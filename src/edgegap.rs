use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::str::FromStr;

pub const PORT_ENV: &str = "ARBITRIUM_PORT_GAMEPORT_INTERNAL";
pub const PUBLIC_IP_ENV: &str = "ARBITRIUM_PUBLIC_IP";
pub const PUBLIC_PORT_ENV: &str = "ARBITRIUM_PORT_GAMEPORT_EXTERNAL";
pub const REQUEST_ID_ENV: &str = "ARBITRIUM_REQUEST_ID";
pub const FQDN_SUFFIX: &str = ".pr.edgegap.net";

#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    pub request_id: String,
    pub public_ip: String,
    pub public_port: u16,
    pub internal_port: u16,
    pub fqdn: String,
}

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

pub fn is_edgegap_server() -> bool {
    std::env::var(PORT_ENV).is_ok()
}

pub fn parse_server_link(link: &str) -> Option<SocketAddr> {
    let link = link.trim();
    if let Ok(addr) = SocketAddr::from_str(link) {
        return Some(addr);
    }
    let (host, port) = link.rsplit_once(':')?;
    let ip = IpAddr::from_str(host).ok()?;
    let port = port.parse().ok()?;
    Some(SocketAddr::new(ip, port))
}

pub fn host_port_from_deployment(deployment: &DeploymentInfo) -> SocketAddr {
    if let Ok(ip) = IpAddr::from_str(&deployment.public_ip) {
        return SocketAddr::new(ip, deployment.public_port);
    }
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), deployment.public_port)
}

fn env_u16(name: &str) -> Option<u16> {
    std::env::var(name).ok()?.parse().ok()
}
