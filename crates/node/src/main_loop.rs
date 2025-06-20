use tracing::{error, info};

use crate::wallet::Wallet;
use crate::{Network, NodeState};
use types::errors::NodeError;
use types::network::network_event::{NetworkEvent, SelfRequest};

impl<N: Network + 'static, W: Wallet + 'static> NodeState<N, W> {
    pub async fn try_poll(&mut self) -> Result<bool, NodeError> {
        let send_message = self.network_events_stream.try_recv().ok();
        if let Some(event) = send_message {
            self.handle(Some(event)).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn poll(&mut self) -> Result<(), NodeError> {
        let send_message = self.network_events_stream.recv().await.ok();
        self.handle(send_message).await
    }

    pub async fn start(&mut self) {
        info!("Local peer id: {}", self.peer_id);

        let mut round_time: tokio::time::Interval =
            tokio::time::interval(std::time::Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = round_time.tick() => {
                    if let Err(e) = self.handle(Some(NetworkEvent::SelfRequest { request: SelfRequest::Tick, response_channel: None })).await {
                        error!("Error handling round tick: {}", e);
                    }
                }
                result = self.poll() => {
                    if let Err(e) = result {
                        error!("Error polling network events: {}", e);
                    }
                }
            }
        }
    }

    pub async fn handle(&mut self, send_message: Option<NetworkEvent>) -> Result<(), NodeError> {
        let mut handlers = std::mem::take(&mut self.handlers);

        for handler in &mut handlers {
            info!("Handling event: {:?}", send_message);
            handler.handle(self, send_message.clone()).await?;
        }

        self.handlers = handlers;
        if let Some(NetworkEvent::PeersConnected(list)) = send_message {
            for (peer_id, _multiaddr) in list {
                self.peers.insert(peer_id);
            }
        }
        Ok(())
    }
}
