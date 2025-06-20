use libp2p::gossipsub::{IdentTopic, Message};
use tracing::info;
use types::errors::NodeError;
use types::intents::DepositIntent;

use crate::{
    NodeState,
    handlers::{Handler, deposit::DepositIntentState},
    wallet::Wallet,
};

use types::network::network_event::{NetworkEvent, SelfRequest, SelfResponse};
use types::network::network_protocol::Network;
use types::proto::ProtoDecode;

#[async_trait::async_trait]
impl<N: Network, W: Wallet> Handler<N, W> for DepositIntentState {
    async fn handle(
        &mut self,
        node: &mut NodeState<N, W>,
        message: Option<NetworkEvent>,
    ) -> Result<(), types::errors::NodeError> {
        match message {
            Some(NetworkEvent::SelfRequest {
                request:
                    SelfRequest::CreateDeposit {
                        user_pubkey,
                        amount_sat,
                    },
                response_channel,
            }) => {
                println!("Node receveived request to create deposit");
                let response = self.create_deposit(node, &user_pubkey, amount_sat).await;
                if let Some(response_channel) = response_channel {
                    match response {
                        Ok((deposit_tracking_id, deposit_address)) => {
                            response_channel
                                .send(SelfResponse::CreateDepositResponse {
                                    deposit_tracking_id,
                                    deposit_address,
                                })
                                .map_err(|e| {
                                    NodeError::Error(format!("Failed to send response: {e}"))
                                })?;
                            println!("Deposit created");
                        }
                        Err(e) => {
                            println!("Error creating deposit: {e:?}, sending response");
                            response_channel
                                .send(SelfResponse::NodeError(e))
                                .map_err(|e| {
                                    NodeError::Error(format!("Failed to send response: {e}"))
                                })?;
                        }
                    }
                }
            }
            Some(NetworkEvent::SelfRequest {
                request: SelfRequest::GetPendingDepositIntents,
                response_channel,
            }) => {
                let response = self.get_pending_deposit_intents(node).await;
                if let Some(response_channel) = response_channel {
                    match response {
                        Ok(intents) => {
                            response_channel
                                .send(SelfResponse::GetPendingDepositIntentsResponse { intents })
                                .map_err(|e| {
                                    NodeError::Error(format!("Failed to send response: {e}"))
                                })?;
                        }
                        Err(e) => {
                            response_channel
                                .send(SelfResponse::NodeError(e))
                                .map_err(|e| {
                                    NodeError::Error(format!("Failed to send response: {e}"))
                                })?;
                        }
                    }
                }
            }
            Some(NetworkEvent::SelfRequest {
                request: SelfRequest::ConfirmDeposit { confirmed_tx },
                ..
            }) => {
                if let Err(e) = self.update_user_balance(node, &confirmed_tx).await {
                    info!("Failed to update user balance: {}", e);
                }
            }
            Some(NetworkEvent::GossipsubMessage(Message { data, topic, .. })) => {
                if topic == IdentTopic::new("deposit-intents").hash() {
                    let deposit_intent = DepositIntent::decode(&data).map_err(|e| {
                        NodeError::Error(format!("Failed to parse deposit intent: {e}"))
                    })?;

                    if let Err(e) = self.create_deposit_from_intent(node, deposit_intent).await {
                        info!("Failed to store deposit intent: {}", e);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
