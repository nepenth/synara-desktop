//! Live MatrixRTC transport owner. Uses `Client::discover_rtc_transports`.

use matrix_sdk::Client;

use super::{map_discovered_rtc_transports, NativeRtcTransportsSnapshot};

enum RtcTransportsSource {
    Client(Client),
    Static(NativeRtcTransportsSnapshot),
}

/// Owns one authenticated session's RTC transport discovery.
pub struct NativeRtcTransportsOwner {
    source: RtcTransportsSource,
    session_generation: u64,
}

impl NativeRtcTransportsOwner {
    pub fn start(client: &Client, session_generation: u64) -> Self {
        let client = client.clone();
        let warmup = client.clone();
        tokio::spawn(async move {
            let _ = warmup.discover_rtc_transports().await;
        });
        Self {
            source: RtcTransportsSource::Client(client),
            session_generation,
        }
    }

    /// Test / fail-closed helper: project a snapshot without a homeserver.
    pub fn from_static_snapshot(
        session_generation: u64,
        snapshot: NativeRtcTransportsSnapshot,
    ) -> Self {
        Self {
            source: RtcTransportsSource::Static(snapshot),
            session_generation,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub async fn snapshot(&self) -> NativeRtcTransportsSnapshot {
        match &self.source {
            RtcTransportsSource::Static(snapshot) => snapshot.clone(),
            RtcTransportsSource::Client(client) => {
                discover(client, self.session_generation, false).await
            }
        }
    }

    pub async fn refresh(&self) -> NativeRtcTransportsSnapshot {
        match &self.source {
            RtcTransportsSource::Static(snapshot) => snapshot.clone(),
            RtcTransportsSource::Client(client) => {
                discover(client, self.session_generation, true).await
            }
        }
    }
}

async fn discover(
    client: &Client,
    session_generation: u64,
    reset: bool,
) -> NativeRtcTransportsSnapshot {
    if reset {
        client.reset_rtc_transports();
    }
    match client.discover_rtc_transports().await {
        Ok(discovered) => map_discovered_rtc_transports(session_generation, discovered),
        Err(_) => NativeRtcTransportsSnapshot::unavailable(session_generation),
    }
}
