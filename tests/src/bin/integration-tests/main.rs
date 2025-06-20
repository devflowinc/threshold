use std::str::FromStr;

use bitcoin::Address;
use bitcoin::secp256k1::{Message, Secp256k1};
use clap::{Parser, Subcommand};
use hex::decode;
use node::key_manager::generate_keys_from_mnemonic;
use node::wallet::{TaprootWallet, Wallet};
use oracle::esplora::EsploraOracle;
use oracle::oracle::Oracle;
use types::proto::node_proto::node_control_client::NodeControlClient;
use types::proto::node_proto::{
    CheckBalanceRequest, ConfirmWithdrawalRequest, CreateDepositIntentRequest,
    ProposeWithdrawalRequest,
};

#[derive(Parser)]
#[command(name = "integration-tests")]
#[command(about = "Run deposit or withdrawal integration flows without bash scripts.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
enum Commands {
    /// Run the deposit integration flow. Requires <amount_sat>. Optional --public-key <hex_pubkey>.
    DepositTest {
        amount: u64,
        #[arg(short, long, default_value_t = 2000)]
        fee: u64,
        #[arg(short, long, default_value_t = false)]
        use_testnet: bool,
        #[arg(short, long)]
        endpoint: Option<String>,
    },
    /// Run the withdrawal integration flow. Requires <amount_sat> <destination_address> <secret_key_hex>
    WithdrawalTest {
        amount: u64,
        #[arg(short, long)]
        endpoint: Option<String>,
    },
    /// Run an end-to-end test of the deposit and withdrawal flows.
    EndToEndTest {
        amount: u64,
        #[arg(short, long, default_value_t = false)]
        use_testnet: bool,
        #[arg(short, long)]
        endpoint: Option<String>,
    },
    CheckDkg {
        #[arg(short, long)]
        ports: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::DepositTest {
            amount,
            fee,
            endpoint,
            use_testnet,
        } => {
            run_deposit_test(amount, fee, endpoint, use_testnet).await?;
        }
        Commands::WithdrawalTest { amount, endpoint } => {
            run_withdrawal_test(amount, endpoint).await?;
        }
        Commands::EndToEndTest {
            amount,
            endpoint,
            use_testnet,
        } => {
            run_end_to_end_test(amount, endpoint, use_testnet).await?;
        }
        Commands::CheckDkg { ports } => {
            check_if_dkg_keys_exist(ports).await?;
        }
    }

    Ok(())
}

async fn run_deposit_test(
    amount: u64,
    fee: u64,
    endpoint: Option<String>,
    use_testnet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("▶️  Creating deposit intent for {} sats", amount);
    let mnemonic = std::env::var("MNEMONIC").expect("MNEMONIC env variable not set");
    let (sender_address, sender_priv, sender_pub) = generate_keys_from_mnemonic(&mnemonic);
    let public_key = sender_pub.to_string();

    let mut client = NodeControlClient::connect(
        endpoint
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:50051".to_string()),
    )
    .await?;

    // Check balance -------------------------------------------
    let request = CheckBalanceRequest {
        address: public_key.clone(),
    };
    let resp = client.check_balance(request).await?.into_inner();
    println!("💰 Initial balance: {} sats", resp.balance_satoshis);
    let initial_balance = resp.balance_satoshis;

    let req = CreateDepositIntentRequest {
        public_key: public_key.clone(),
        amount_satoshis: amount,
    };

    let resp = client.create_deposit_intent(req).await?.into_inner();
    let deposit_address_str = resp.deposit_address.clone();
    println!(
        "✅ Deposit intent created. Address: {} | tracking_id: {}",
        deposit_address_str, resp.deposit_tracking_id
    );

    if use_testnet {
        println!("🔑 Sender address: {}. Refreshing UTXOs...", sender_address);

        let oracle = EsploraOracle::new(bitcoin::Network::Testnet, Some(100), None, None, 6, 0);
        let mut wallet = TaprootWallet::new(
            Box::new(oracle.clone()),
            vec![sender_address.clone()],
            bitcoin::Network::Testnet,
        );
        wallet.refresh_utxos(Some(true)).await?;

        let deposit_address = Address::from_str(&deposit_address_str)?.assume_checked();
        let (tx, sighash) = wallet.create_spend(amount, fee, &deposit_address, false)?;
        let txid = tx.compute_txid();
        let signed_tx = wallet.sign(&tx, &sender_priv, sighash);

        oracle.broadcast_transaction(&signed_tx).await?;
        println!("📤 Broadcast Transaction txid: {}", txid);

        let start_time = std::time::Instant::now();
        loop {
            let request = CheckBalanceRequest {
                address: public_key.clone(),
            };
            let resp = client.check_balance(request).await?.into_inner();
            println!("💰 Final balance: {} sats", resp.balance_satoshis);

            if resp.balance_satoshis == initial_balance + amount {
                break;
            }

            if start_time.elapsed() >= std::time::Duration::from_secs(1000) {
                println!("⏰ Timeout reached. Exiting polling loop.");
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    } else {
        let request = CheckBalanceRequest {
            address: public_key.clone(),
        };
        let resp = client.check_balance(request).await?.into_inner();
        println!("💰 Final balance: {} sats", resp.balance_satoshis);

        assert_eq!(
            resp.balance_satoshis,
            initial_balance + amount,
            "Balance after deposit intent should be equal to initial balance + amount"
        );
    }
    println!("✅ Deposit test passed");
    Ok(())
}

async fn run_withdrawal_test(
    amount: u64,
    endpoint: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mnemonic = std::env::var("MNEMONIC").expect("MNEMONIC env variable not set");
    let (_, sender_priv, sender_pub) = generate_keys_from_mnemonic(&mnemonic);
    let secp = Secp256k1::new();
    let public_key = sender_pub.to_string();

    // Propose withdrawal --------------------------------------
    let mut client = NodeControlClient::connect(
        endpoint
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:50051".to_string()),
    )
    .await?;

    // Check balance -------------------------------------------
    let request = CheckBalanceRequest {
        address: public_key.clone(),
    };
    let resp = client.check_balance(request).await?.into_inner();
    println!("💰 Initial balance: {} sats", resp.balance_satoshis);
    let initial_balance = resp.balance_satoshis;

    let deposit_address = Address::p2wpkh(&sender_pub, bitcoin::Network::Testnet).to_string();

    let req = ProposeWithdrawalRequest {
        amount_satoshis: amount,
        address_to: deposit_address.clone(),
        public_key: public_key.clone(),
        blocks_to_confirm: None,
    };

    let propose_resp = client.propose_withdrawal(req).await?.into_inner();
    let challenge_hex = propose_resp.challenge;
    println!(
        "📝 Withdrawal proposed. Challenge: {} | quote: {} sats",
        challenge_hex, propose_resp.quote_satoshis
    );

    // Sign challenge ------------------------------------------
    let challenge_bytes = decode(&challenge_hex)?;
    let msg = Message::from_digest_slice(&challenge_bytes)?;
    let signature = secp.sign_ecdsa(&msg, &sender_priv.inner);
    let signature_hex = hex::encode(signature.serialize_der());
    println!("✍️  Signature: {}", signature_hex);

    // Confirm withdrawal --------------------------------------
    let confirm_req = ConfirmWithdrawalRequest {
        challenge: challenge_hex.clone(),
        signature: signature_hex.clone(),
    };
    let confirm_resp = client.confirm_withdrawal(confirm_req).await?.into_inner();
    println!(
        "✅ Withdrawal confirmation success: {}",
        confirm_resp.success
    );

    // Wait for withdrawal to be processed --------------------------------------
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Check balance -------------------------------------------
    let request = CheckBalanceRequest {
        address: public_key.clone(),
    };
    let resp = client.check_balance(request).await?.into_inner();
    println!("💰 Final balance: {} sats", resp.balance_satoshis);
    assert_eq!(
        resp.balance_satoshis,
        initial_balance - propose_resp.quote_satoshis,
        "Balance after withdrawal should be equal to initial balance - quoted amount"
    );

    println!("✅ Withdrawal test passed");
    Ok(())
}

async fn run_end_to_end_test(
    amount: u64,
    endpoint: Option<String>,
    use_testnet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_deposit_test(amount, 2000, endpoint.clone(), use_testnet).await?;
    run_withdrawal_test(amount / 2, endpoint).await?;
    println!("✅ End-to-end test passed");
    Ok(())
}

async fn check_if_dkg_keys_exist(ports: String) -> Result<(), Box<dyn std::error::Error>> {
    for port in ports.split(",") {
        let mut client = NodeControlClient::connect(format!("http://127.0.0.1:{port}")).await?;

        let mnemonic = std::env::var("MNEMONIC").expect("MNEMONIC env variable not set");
        let (_, _, sender_pub) = generate_keys_from_mnemonic(&mnemonic);
        let public_key = sender_pub.to_string();

        let req = CreateDepositIntentRequest {
            public_key: public_key.clone(),
            amount_satoshis: 1000,
        };

        let response = client.create_deposit_intent(req).await;

        match response {
            Ok(_) => {}
            Err(e) => {
                panic!("Deposit intent creation failed for node on port {port}: {e}");
            }
        }
    }

    println!("✅ DKG keys exist");

    Ok(())
}
