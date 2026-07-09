use core::net::{IpAddr, SocketAddr};
use core::str::FromStr;
use std::net::ToSocketAddrs;

use rustls_tokio_postgres::{config_no_verify, MakeRustlsConnect};
use tokio_postgres::Client;

pub const USE_SERVER_DISCOVERY: bool = true;

const DATABASE_URL_FILE: &str = "secrets/database-client-url.txt";

#[derive(Debug, Clone)]
pub struct DiscoveredServer {
    pub host: String,
    pub port: u16,
}

pub fn discover_latest_server() -> Result<DiscoveredServer, String> {
    let url = database_url()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime failed: {e}"))?;
    rt.block_on(discover_async(&url))
}

async fn discover_async(url: &str) -> Result<DiscoveredServer, String> {
    let client = connect(url).await?;
    let row = client
        .query_one("SELECT host, port FROM get_latest_server()", &[])
        .await
        .map_err(|e| format!("get_latest_server failed: {e}"))?;
    let port: i32 = row.get(1);
    Ok(DiscoveredServer {
        host: row.get(0),
        port: port as u16,
    })
}

async fn connect(url: &str) -> Result<Client, String> {
    if needs_tls(url) {
        let connector = MakeRustlsConnect::new(config_no_verify());
        let (client, connection) = tokio_postgres::connect(url, connector)
            .await
            .map_err(|e| format!("database connect failed: {e}"))?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("postgres connection error: {e}");
            }
        });
        return Ok(client);
    }

    let (client, connection) = tokio_postgres::connect(url, tokio_postgres::NoTls)
        .await
        .map_err(|e| format!("database connect failed: {e}"))?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("postgres connection error: {e}");
        }
    });
    Ok(client)
}

pub fn resolve_server_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
    if let Ok(ip) = IpAddr::from_str(host.trim()) {
        return Ok(SocketAddr::new(ip, port));
    }
    let endpoint = format!("{}:{port}", host.trim());
    endpoint
        .to_socket_addrs()
        .map_err(|e| format!("failed to resolve {endpoint}: {e}"))?
        .next()
        .ok_or_else(|| format!("no addresses resolved for {endpoint}"))
}

fn database_url() -> Result<String, String> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return Ok(url);
    }
    std::fs::read_to_string(DATABASE_URL_FILE)
        .map(|url| url.trim().to_string())
        .map_err(|e| {
            format!(
                "set DATABASE_URL or create {DATABASE_URL_FILE} with the client connection string: {e}"
            )
        })
}

fn needs_tls(url: &str) -> bool {
    url.contains("sslmode=require")
        || url.contains("sslmode=verify-full")
        || url.contains("sslmode=verify-ca")
        || url.contains("ondigitalocean.com")
}
