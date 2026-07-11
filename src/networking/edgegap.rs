#[cfg(feature = "server")]
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
#[cfg(feature = "server")]
use core::str::FromStr;

#[cfg(feature = "server")]
/// Edgegap environment variable containing the internal game port.
const PORT_ENV: &str = "ARBITRIUM_PORT_GAMEPORT_INTERNAL";
#[cfg(feature = "server")]
/// Edgegap environment variable containing the public IP or host.
const PUBLIC_IP_ENV: &str = "ARBITRIUM_PUBLIC_IP";
#[cfg(feature = "server")]
/// Edgegap environment variable containing the public game port.
const PUBLIC_PORT_ENV: &str = "ARBITRIUM_PORT_GAMEPORT_EXTERNAL";
#[cfg(feature = "server")]
/// Edgegap environment variable containing the deployment request id.
const REQUEST_ID_ENV: &str = "ARBITRIUM_REQUEST_ID";

#[cfg(feature = "server")]
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
}

#[cfg(feature = "server")]
/// Reads the Edgegap deployment environment, if this process is running there.
pub fn deployment_info() -> Option<DeploymentInfo> {
    let internal_port = env_u16(PORT_ENV)?;
    let request_id = std::env::var(REQUEST_ID_ENV).ok()?;
    let public_ip = std::env::var(PUBLIC_IP_ENV).unwrap_or_else(|_| request_id.clone());
    let public_port = env_u16(PUBLIC_PORT_ENV).unwrap_or(internal_port);
    Some(DeploymentInfo {
        request_id,
        public_ip,
        public_port,
        internal_port,
    })
}

#[cfg(feature = "server")]
/// Returns whether the process appears to be running in Edgegap.
pub fn is_edgegap_server() -> bool {
    std::env::var(PORT_ENV).is_ok()
}

#[cfg(feature = "server")]
/// Converts Edgegap public connection metadata into a socket address.
pub fn host_port_from_deployment(deployment: &DeploymentInfo) -> SocketAddr {
    if let Ok(ip) = IpAddr::from_str(&deployment.public_ip) {
        return SocketAddr::new(ip, deployment.public_port);
    }
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), deployment.public_port)
}

#[cfg(feature = "server")]
/// Reads and parses an environment variable as a `u16`.
fn env_u16(name: &str) -> Option<u16> {
    std::env::var(name).ok()?.parse().ok()
}

#[cfg(feature = "client")]
pub use discovery::*;

#[cfg(feature = "client")]
/// Client-side Edgegap deploy/list/poll discovery.
mod discovery {
    use core::net::{IpAddr, SocketAddr};
    use core::str::FromStr;
    use core::time::Duration;
    use std::collections::HashMap;
    #[cfg(not(target_arch = "wasm32"))]
    use std::net::ToSocketAddrs;

    use bevy::prelude::*;
    use bevy_mod_reqwest::{BevyReqwest, ReqwestErrorEvent, ReqwestResponseEvent};
    use serde::Deserialize;
    use serde::Serialize;

    /// DNS suffix used for Edgegap request-id hostnames.
    const FQDN_SUFFIX: &str = ".pr.edgegap.net";

    /// Base URL for Edgegap's public API.
    const API_BASE: &str = "https://api.edgegap.com";
    /// Edgegap API token used for deployment discovery.
    const API_TOKEN: &str = "token 6d12964d-c588-4e26-9810-fe5ad00b33af";
    /// Edgegap application name to deploy.
    const APP_NAME: &str = "inventory-jam";
    /// Edgegap application version to deploy.
    const APP_VERSION: &str = "dev";
    /// Edgegap port name that carries gameplay traffic.
    const GAME_PORT_NAME: &str = "gameport";
    /// Latitude used for the development deployment request.
    const LATITUDE: f64 = 45.5231;
    /// Longitude used for the development deployment request.
    const LONGITUDE: f64 = -122.6765;
    /// Delay between Edgegap status requests.
    const POLL_INTERVAL_SECS: u64 = 5;
    /// Maximum time to wait for an Edgegap deployment to become ready.
    const POLL_TIMEOUT_SECS: u64 = 600;

    /// Host and port resolved from Edgegap deployment data.
    #[derive(Debug, Clone)]
    pub struct DiscoveredServer {
        /// Public host or IP returned by Edgegap.
        pub host: String,
        /// Public game port returned by Edgegap.
        pub port: u16,
    }

    /// Current state of remote server discovery.
    #[derive(Debug, Clone)]
    pub enum ServerDiscoveryState {
        /// Discovery is still making Edgegap requests.
        Discovering,
        /// Discovery resolved a server socket address.
        Ready(SocketAddr),
        /// Discovery failed with an error message.
        Failed(String),
    }

    /// Bevy resource storing the current discovery result.
    #[derive(Resource)]
    pub struct ServerDiscovery {
        /// Current discovery state observed by networking systems.
        pub state: ServerDiscoveryState,
    }

    impl ServerDiscovery {
        /// Creates a discovery resource in the initial discovering state.
        pub const fn new() -> Self {
            Self {
                state: ServerDiscoveryState::Discovering,
            }
        }

        /// Records a terminal discovery failure.
        fn fail(&mut self, error: impl Into<String>) {
            self.state = ServerDiscoveryState::Failed(error.into());
        }

        /// Resolves and records a ready server address.
        fn set_ready(&mut self, server: DiscoveredServer) {
            match resolve_server_addr(&server.host, server.port) {
                Ok(addr) => {
                    self.state = ServerDiscoveryState::Ready(addr);
                }
                Err(error) => self.fail(error),
            }
        }
    }

    /// Edgegap API request currently being sent or scheduled.
    #[derive(Debug, Clone)]
    enum EdgegapRequest {
        /// Lists recent deployments to reuse one if possible.
        ListDeployments,
        /// Creates a new deployment.
        CreateDeployment,
        /// Checks readiness for a deployment request id.
        DeploymentStatus {
            /// Edgegap deployment request id.
            request_id: String,
            /// Number of status checks remaining before timeout.
            remaining_polls: u64,
        },
    }

    impl EdgegapRequest {
        /// Returns the API path for this request.
        fn path(&self) -> String {
            match self {
                Self::ListDeployments => "/v1/deployments?limit=50".to_string(),
                Self::CreateDeployment => "/v2/deployments".to_string(),
                Self::DeploymentStatus { request_id, .. } => format!("/v1/status/{request_id}"),
            }
        }
    }

    /// Delayed status request marker used between Edgegap polling attempts.
    #[derive(Component)]
    pub struct PendingStatusRequest {
        /// Edgegap deployment request id to check.
        request_id: String,
        /// Number of remaining status checks before timeout.
        remaining_polls: u64,
        /// Timer delaying the next status request.
        timer: Timer,
    }

    /// Sends the initial Edgegap discovery request.
    pub fn start_server_discovery(
        discovery: Option<ResMut<ServerDiscovery>>,
        mut client: BevyReqwest,
    ) {
        let Some(mut discovery) = discovery else {
            return;
        };
        if !matches!(discovery.state, ServerDiscoveryState::Discovering) {
            return;
        }
        if let Err(error) = send_edgegap_request(&mut client, EdgegapRequest::ListDeployments) {
            discovery.fail(error);
        }
    }

    /// Sends delayed deployment status requests once their timers expire.
    pub fn send_due_status_requests(
        mut commands: Commands,
        discovery: Option<ResMut<ServerDiscovery>>,
        time: Res<Time>,
        mut client: BevyReqwest,
        mut pending_requests: Query<(Entity, &mut PendingStatusRequest)>,
    ) {
        let Some(mut discovery) = discovery else {
            return;
        };
        for (entity, mut pending) in pending_requests.iter_mut() {
            if !matches!(discovery.state, ServerDiscoveryState::Discovering) {
                commands.entity(entity).despawn();
                continue;
            }
            pending.timer.tick(time.delta());
            if !pending.timer.is_finished() {
                continue;
            }
            let request = EdgegapRequest::DeploymentStatus {
                request_id: pending.request_id.clone(),
                remaining_polls: pending.remaining_polls,
            };
            if let Err(error) = send_edgegap_request(&mut client, request) {
                discovery.fail(error);
            }
            commands.entity(entity).despawn();
        }
    }

    /// Builds and sends an Edgegap request with the matching response observer.
    fn send_edgegap_request(
        client: &mut BevyReqwest,
        edgegap_request: EdgegapRequest,
    ) -> Result<(), String> {
        let url = format!("{API_BASE}{}", edgegap_request.path());
        let request = build_edgegap_request(client, &edgegap_request, &url)?;
        match edgegap_request {
            EdgegapRequest::ListDeployments => {
                client
                    .send(request)
                    .on_response(handle_list_deployments_response)
                    .on_error(handle_edgegap_error);
            }
            EdgegapRequest::CreateDeployment => {
                client
                    .send(request)
                    .on_response(handle_create_deployment_response)
                    .on_error(handle_edgegap_error);
            }
            EdgegapRequest::DeploymentStatus {
                request_id,
                remaining_polls,
            } => {
                client
                    .send(request)
                    .on_response(
                        move |trigger: On<ReqwestResponseEvent>,
                              discovery: ResMut<ServerDiscovery>,
                              commands: Commands| {
                            handle_deployment_status_response(
                                trigger,
                                discovery,
                                commands,
                                request_id.clone(),
                                remaining_polls,
                            );
                        },
                    )
                    .on_error(handle_edgegap_error);
            }
        }
        Ok(())
    }

    /// Resolves a host and port into a socket address.
    pub fn resolve_server_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
        if let Ok(ip) = IpAddr::from_str(host.trim()) {
            return Ok(SocketAddr::new(ip, port));
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err(format!(
                "wasm requires an IP address for the server, got host {host}"
            ))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let endpoint = format!("{}:{port}", host.trim());
            endpoint
                .to_socket_addrs()
                .map_err(|e| format!("failed to resolve {endpoint}: {e}"))?
                .next()
                .ok_or_else(|| format!("no addresses resolved for {endpoint}"))
        }
    }

    /// Builds the HTTP request for an Edgegap API operation.
    fn build_edgegap_request(
        client: &BevyReqwest,
        edgegap_request: &EdgegapRequest,
        url: &str,
    ) -> Result<bevy_mod_reqwest::reqwest::Request, String> {
        let builder = match edgegap_request {
            EdgegapRequest::ListDeployments | EdgegapRequest::DeploymentStatus { .. } => {
                client.get(url)
            }
            EdgegapRequest::CreateDeployment => {
                let body = serde_json::to_string(&CreateDeploymentBody {
                    application: APP_NAME,
                    version: APP_VERSION,
                    users: vec![DeploymentUser {
                        user_type: "geo_coordinates",
                        user_data: GeoCoordinates {
                            latitude: LATITUDE,
                            longitude: LONGITUDE,
                        },
                    }],
                })
                .map_err(|e| format!("deployment json failed: {e}"))?;
                client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .body(body)
            }
        };
        builder
            .header("Authorization", API_TOKEN)
            .header("Accept", "application/json")
            .build()
            .map_err(|e| format!("failed to build edgegap request to {url}: {e}"))
    }

    /// Handles the deployment list response and chooses reuse or creation.
    fn handle_list_deployments_response(
        trigger: On<ReqwestResponseEvent>,
        mut discovery: ResMut<ServerDiscovery>,
        mut client: BevyReqwest,
    ) {
        let response = trigger.event();
        let path = EdgegapRequest::ListDeployments.path();
        let list = match parse_edgegap_response::<DeploymentList>(response, &path) {
            Ok(list) => list,
            Err(error) => {
                discovery.fail(error);
                return;
            }
        };
        match find_existing_deployment(list) {
            Ok(Some(ExistingDeployment::Ready(server))) => discovery.set_ready(server),
            Ok(Some(ExistingDeployment::Pending(request_id))) => {
                let request = EdgegapRequest::DeploymentStatus {
                    request_id,
                    remaining_polls: POLL_TIMEOUT_SECS.div_ceil(POLL_INTERVAL_SECS),
                };
                if let Err(error) = send_edgegap_request(&mut client, request) {
                    discovery.fail(error);
                }
            }
            Ok(None) => {
                if let Err(error) =
                    send_edgegap_request(&mut client, EdgegapRequest::CreateDeployment)
                {
                    discovery.fail(error);
                }
            }
            Err(error) => discovery.fail(error),
        }
    }

    /// Handles deployment creation and starts status polling.
    fn handle_create_deployment_response(
        trigger: On<ReqwestResponseEvent>,
        mut discovery: ResMut<ServerDiscovery>,
        mut client: BevyReqwest,
    ) {
        let response = trigger.event();
        let path = EdgegapRequest::CreateDeployment.path();
        let response = match parse_edgegap_response::<CreateDeploymentResponse>(response, &path) {
            Ok(response) => response,
            Err(error) => {
                discovery.fail(error);
                return;
            }
        };
        if let Err(error) = send_edgegap_request(
            &mut client,
            EdgegapRequest::DeploymentStatus {
                request_id: response.request_id,
                remaining_polls: POLL_TIMEOUT_SECS.div_ceil(POLL_INTERVAL_SECS),
            },
        ) {
            discovery.fail(error);
        }
    }

    /// Handles a deployment status response.
    fn handle_deployment_status_response(
        trigger: On<ReqwestResponseEvent>,
        mut discovery: ResMut<ServerDiscovery>,
        mut commands: Commands,
        request_id: String,
        remaining_polls: u64,
    ) {
        let response = trigger.event();
        let path = EdgegapRequest::DeploymentStatus {
            request_id: request_id.clone(),
            remaining_polls,
        }
        .path();
        let status = match parse_edgegap_response::<DeploymentStatus>(response, &path) {
            Ok(status) => status,
            Err(error) => {
                discovery.fail(error);
                return;
            }
        };
        if status.error == Some(true) {
            discovery.fail(format!(
                "edgegap deployment {request_id} failed: {}",
                status
                    .error_detail
                    .unwrap_or_else(|| "unknown error".into())
            ));
            return;
        }
        if status.running {
            match connection_from_status(&status) {
                Ok(server) => discovery.set_ready(server),
                Err(error) => discovery.fail(error),
            }
            return;
        }
        if remaining_polls <= 1 {
            discovery.fail(format!(
                "timed out waiting for edgegap deployment {request_id} to become ready"
            ));
            return;
        }
        commands.spawn(PendingStatusRequest {
            request_id,
            remaining_polls: remaining_polls - 1,
            timer: Timer::new(Duration::from_secs(POLL_INTERVAL_SECS), TimerMode::Once),
        });
    }

    /// Handles transport-level Edgegap request failures.
    fn handle_edgegap_error(
        trigger: On<ReqwestErrorEvent>,
        mut discovery: ResMut<ServerDiscovery>,
    ) {
        discovery.fail(format!("edgegap request failed: {}", trigger.event().error));
    }

    /// Parses a successful Edgegap response body as JSON.
    fn parse_edgegap_response<T: for<'de> Deserialize<'de>>(
        response: &ReqwestResponseEvent,
        path: &str,
    ) -> Result<T, String> {
        let text = response
            .as_str()
            .map_err(|e| format!("edgegap response body from {API_BASE}{path} failed: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "edgegap {API_BASE}{path} returned {}: {text}",
                response.status()
            ));
        }
        serde_json::from_str(text)
            .map_err(|e| format!("invalid edgegap response from {path}: {e}\n{text}"))
    }

    /// Existing deployment state returned from list responses.
    enum ExistingDeployment {
        /// Deployment is ready and has connection info.
        Ready(DiscoveredServer),
        /// Deployment exists but still needs status polling.
        Pending(String),
    }

    /// Finds a reusable deployment or pending request id from a deployment list.
    fn find_existing_deployment(
        list: DeploymentList,
    ) -> Result<Option<ExistingDeployment>, String> {
        let mut pending = None;
        for deployment in list.data {
            if deployment.ready {
                return Ok(Some(ExistingDeployment::Ready(connection_from_deployment(
                    &deployment,
                )?)));
            }
            if pending.is_none() {
                pending = Some(deployment.request_id);
            }
        }
        Ok(pending.map(ExistingDeployment::Pending))
    }

    /// Extracts connection info from a ready deployment summary.
    fn connection_from_deployment(
        deployment: &DeploymentSummary,
    ) -> Result<DiscoveredServer, String> {
        let port = deployment
            .ports
            .as_ref()
            .and_then(|ports| ports.get(GAME_PORT_NAME))
            .and_then(|port| port.external.as_ref().and_then(parse_port_value))
            .ok_or_else(|| format!("deployment missing port '{GAME_PORT_NAME}'"))?;

        let host = deployment
            .public_ip
            .clone()
            .or_else(|| deployment.fqdn.clone())
            .unwrap_or_else(|| format!("{}{FQDN_SUFFIX}", deployment.request_id));
        Ok(DiscoveredServer { host, port })
    }

    /// Extracts connection info from a running deployment status.
    fn connection_from_status(status: &DeploymentStatus) -> Result<DiscoveredServer, String> {
        let port = status
            .ports
            .as_ref()
            .and_then(|ports| ports.get(GAME_PORT_NAME))
            .and_then(|port| port.external.as_ref().and_then(parse_port_value))
            .ok_or_else(|| format!("edgegap status missing port '{GAME_PORT_NAME}'"))?;

        let host = status
            .public_ip
            .clone()
            .or_else(|| {
                status
                    .request_id
                    .as_ref()
                    .map(|id| format!("{id}{FQDN_SUFFIX}"))
            })
            .ok_or_else(|| "edgegap status missing host".to_string())?;
        Ok(DiscoveredServer { host, port })
    }

    /// Parses Edgegap port values represented as either numbers or strings.
    fn parse_port_value(value: &serde_json::Value) -> Option<u16> {
        match value {
            serde_json::Value::Number(number) => {
                number.as_u64().and_then(|port| u16::try_from(port).ok())
            }
            serde_json::Value::String(text) => text.parse().ok(),
            _ => None,
        }
    }

    /// Response body for the deployment list endpoint.
    #[derive(Deserialize)]
    struct DeploymentList {
        /// Deployment summaries returned by Edgegap.
        data: Vec<DeploymentSummary>,
    }

    /// Summary entry returned by the deployment list endpoint.
    #[derive(Deserialize)]
    struct DeploymentSummary {
        /// Edgegap deployment request id.
        request_id: String,
        #[serde(default)]
        /// Optional fully qualified domain name for the deployment.
        fqdn: Option<String>,
        #[serde(default)]
        /// Whether the deployment is ready to accept connections.
        ready: bool,
        #[serde(default)]
        /// Optional public IP or hostname for the deployment.
        public_ip: Option<String>,
        #[serde(default)]
        /// Port map keyed by Edgegap port name.
        ports: Option<HashMap<String, PortInfo>>,
    }

    /// Response body for a deployment status request.
    #[derive(Deserialize)]
    struct DeploymentStatus {
        /// Whether the deployment is running.
        running: bool,
        #[serde(default)]
        /// Whether Edgegap reported a deployment error.
        error: Option<bool>,
        #[serde(default)]
        /// Optional error detail from Edgegap.
        error_detail: Option<String>,
        #[serde(default)]
        /// Optional public IP or hostname for the deployment.
        public_ip: Option<String>,
        #[serde(default)]
        /// Optional deployment request id.
        request_id: Option<String>,
        #[serde(default)]
        /// Port map keyed by Edgegap port name.
        ports: Option<HashMap<String, PortInfo>>,
    }

    /// Edgegap port metadata.
    #[derive(Deserialize)]
    struct PortInfo {
        #[serde(default)]
        /// External/public port value.
        external: Option<serde_json::Value>,
    }

    /// Response body returned when creating a deployment.
    #[derive(Deserialize)]
    struct CreateDeploymentResponse {
        /// Edgegap deployment request id.
        request_id: String,
    }

    /// Request body sent when creating a deployment.
    #[derive(Serialize)]
    struct CreateDeploymentBody {
        /// Edgegap application name.
        application: &'static str,
        /// Edgegap application version.
        version: &'static str,
        /// Users used by Edgegap matchmaking placement.
        users: Vec<DeploymentUser>,
    }

    /// User placement entry for Edgegap deployment creation.
    #[derive(Serialize)]
    struct DeploymentUser {
        /// Edgegap user data type.
        user_type: &'static str,
        /// User placement data.
        user_data: GeoCoordinates,
    }

    /// Geographic placement data for Edgegap deployment creation.
    #[derive(Serialize)]
    struct GeoCoordinates {
        /// Latitude used for placement.
        latitude: f64,
        /// Longitude used for placement.
        longitude: f64,
    }
}
