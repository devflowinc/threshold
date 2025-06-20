use bitcoin::Transaction;
use frost_secp256k1::keys::dkg::round2;
use libp2p::{
    Multiaddr, PeerId,
    gossipsub::{self},
};
use tokio::sync::mpsc;

use crate::intents::{DepositIntent, WithdrawlIntent};

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    SelfRequest {
        request: SelfRequest,
        response_channel: Option<mpsc::UnboundedSender<SelfResponse>>,
    },
    Subscribed {
        peer_id: PeerId,
        topic: gossipsub::TopicHash,
    },
    GossipsubMessage(gossipsub::Message),
    MessageEvent((PeerId, DirectMessage)),
    PeersConnected(Vec<(PeerId, Multiaddr)>),
    PeersDisconnected(Vec<(PeerId, Multiaddr)>),
    Unknown,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PingBody {
    pub message: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum DirectMessage {
    Ping(PingBody),
    Round2Package(round2::Package),
    SignRequest {
        sign_id: u64,
        message: Vec<u8>,
    },
    SignPackage {
        sign_id: u64,
        package: Vec<u8>,
    },
    Pong,
    Commitments {
        sign_id: u64,
        commitments: Vec<u8>,
    },
    SignatureShare {
        sign_id: u64,
        signature_share: Vec<u8>,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SelfRequest {
    CreateDeposit {
        user_pubkey: String,
        amount_sat: u64,
    },
    GetPendingDepositIntents,
    StartSigningSession {
        hex_message: String,
    },
    Spend {
        amount_sat: u64,
        fee: u64,
        address_to: String,
        user_pubkey: String,
    },
    ProposeWithdrawal {
        withdrawal_intent: WithdrawlIntent,
    },
    ConfirmWithdrawal {
        challenge: String,
        signature: String,
    },
    CheckBalance {
        address: String,
    },
    ConfirmDeposit {
        confirmed_tx: Transaction,
    },
    Tick,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SelfResponse {
    GetFrostPublicKeyResponse {
        public_key: Option<String>,
    },
    CreateDepositResponse {
        deposit_tracking_id: String,
        deposit_address: String,
    },
    GetPendingDepositIntentsResponse {
        intents: Vec<DepositIntent>,
    },
    StartSigningSessionResponse {
        sign_id: u64,
    },
    SpendRequestSent {
        sighash: String,
    },
    ProposeWithdrawalResponse {
        quote_satoshis: u64,
        challenge: String,
    },
    ConfirmWithdrawalResponse {
        success: bool,
    },
    CheckBalanceResponse {
        balance_satoshis: u64,
    },
    NodeError(crate::errors::NodeError),
}
