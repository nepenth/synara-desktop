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
    warmup: Option<tokio::task::JoinHandle<()>>,
}

impl NativeRtcTransportsOwner {
    pub fn start(client: &Client, session_generation: u64) -> Self {
        let client = client.clone();
        let warmup = client.clone();
        let warmup = tokio::spawn(async move {
            let _ = warmup.discover_rtc_transports().await;
        });
        Self {
            source: RtcTransportsSource::Client(client),
            session_generation,
            warmup: Some(warmup),
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
            warmup: None,
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

impl Drop for NativeRtcTransportsOwner {
    fn drop(&mut self) {
        // JoinHandle drop detaches. Abort the session-owned discovery future so
        // retirement cannot leave a native warmup running with the old client.
        if let Some(warmup) = &self.warmup {
            warmup.abort();
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

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::{config::RequestConfig, test_utils::mocks::MatrixMockServer};
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use wiremock::{
        matchers::{method, path_regex},
        Mock, ResponseTemplate,
    };

    #[tokio::test]
    async fn dropping_rtc_owner_aborts_inflight_sdk_warmup() {
        let server = MatrixMockServer::new().await;
        let client = server
            .client_builder()
            .on_builder(|builder| builder.request_config(RequestConfig::new().disable_retry()))
            .build()
            .await;
        let (started_send, started_receive) = tokio::sync::oneshot::channel();
        let started_send = Arc::new(Mutex::new(Some(started_send)));
        Mock::given(method("GET"))
            .and(path_regex(r"/rtc/transports$"))
            .respond_with(move |_: &wiremock::Request| {
                if let Some(sender) = started_send.lock().unwrap().take() {
                    let _ = sender.send(());
                }
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"rtc_transports": []}))
                    .set_delay(Duration::from_secs(60))
            })
            .mount(server.server())
            .await;
        let owner = NativeRtcTransportsOwner::start(&client, 7);
        let abort = owner.warmup.as_ref().unwrap().abort_handle();
        tokio::time::timeout(Duration::from_secs(5), started_receive)
            .await
            .unwrap()
            .unwrap();
        assert!(
            !abort.is_finished(),
            "SDK warmup is awaiting its delayed response"
        );
        drop(owner);
        tokio::time::timeout(Duration::from_secs(1), async {
            while !abort.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("owner retirement aborts the actual pending SDK future");
    }
}
