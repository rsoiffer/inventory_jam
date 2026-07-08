//! This module contains the shared code between the client and the server.

use bevy::prelude::*;
#[cfg(all(feature = "webtransport", not(target_family = "wasm")))]
use bevy::tasks::IoTaskPool;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::time::Duration;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::{Inputs, PlayerPosition, ProtocolPlugin};

pub const FIXED_TIMESTEP_HZ: f64 = 64.0;

pub const SERVER_REPLICATION_INTERVAL: Duration = Duration::from_millis(100);

pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5000);

/// Netcode.io protocol id and private key. These just need to match between client and server;
/// a zeroed key is fine here since we authenticate manually instead of via a backend.
pub const PROTOCOL_ID: u64 = 0;
pub const PRIVATE_KEY: [u8; 32] = [0; 32];

#[cfg(all(feature = "webtransport", not(target_family = "wasm")))]
pub(crate) fn webtransport_self_signed_certificate() -> Identity {
    // Keep this file-backed so native servers and wasm clients agree on certificates/digest.txt.
    // Runtime-generated self-signed identities have a new digest each run.
    let cert = format!("{}/../../certificates/cert.pem", env!("CARGO_MANIFEST_DIR"));
    let key = format!("{}/../../certificates/key.pem", env!("CARGO_MANIFEST_DIR"));
    IoTaskPool::get()
        .scope(|s| {
            s.spawn(async_compat::Compat::new(async move {
                Identity::load_pemfiles(&cert, &key).await.unwrap()
            }));
        })
        .pop()
        .unwrap()
}

#[derive(Clone)]
pub struct SharedPlugin;

pub struct Channel1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message1(pub usize);

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);
    }
}

// This system defines how we update the player's positions when we receive an input
pub(crate) fn shared_movement_behaviour(mut position: Mut<PlayerPosition>, input: &Inputs) {
    const MOVE_SPEED: f32 = 10.0;
    let direction = &input.direction;
    if direction.up {
        position.y += MOVE_SPEED;
    }
    if direction.down {
        position.y -= MOVE_SPEED;
    }
    if direction.left {
        position.x -= MOVE_SPEED;
    }
    if direction.right {
        position.x += MOVE_SPEED;
    }
}
