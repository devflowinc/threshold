use clap::{Parser, Subcommand};
use node::grpc::grpc_handler::node_proto::{
    node_control_client::NodeControlClient, ConfirmWithdrawalRequest, ProposeWithdrawalRequest,
};

#[derive(Parser)]
#[command(name = "withdrawal")]
#[command(about = "Handle withdrawal operations for TheVault")]
#[command(version = "0.0.1")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Propose a withdrawal and get a challenge to sign
    Propose {
        amount: u64,
        address_to: String,
        public_key: String,
        #[arg(short, long)]
        endpoint: Option<String>,
    },
    /// Confirm a withdrawal with a signature
    Confirm {
        challenge: String,
        signature: String,
        #[arg(short, long)]
        endpoint: Option<String>,
    },
}

async fn propose_withdrawal(
    endpoint: Option<String>,
    amount: u64,
    address_to: String,
    public_key: String,
) -> Result<(), String> {
    let endpoint = endpoint.unwrap_or_else(|| "http://127.0.0.1:50051".to_string());
    let mut client = NodeControlClient::connect(endpoint)
        .await
        .map_err(|e| e.to_string())?;

    let request = ProposeWithdrawalRequest {
        amount_satoshis: amount,
        address_to,
        public_key,
        blocks_to_confirm: None,
    };

    let response = client
        .propose_withdrawal(request)
        .await
        .map_err(|e| e.to_string())?;

    let response = response.into_inner();
    println!("Withdrawal proposed successfully");
    println!("Challenge: {}", response.challenge);
    println!("Quote amount: {} satoshis", response.quote_satoshis);

    Ok(())
}

async fn confirm_withdrawal(
    endpoint: Option<String>,
    challenge: String,
    signature: String,
) -> Result<(), String> {
    let endpoint = endpoint.unwrap_or_else(|| "http://127.0.0.1:50051".to_string());
    let mut client = NodeControlClient::connect(endpoint)
        .await
        .map_err(|e| e.to_string())?;

    let request = ConfirmWithdrawalRequest {
        challenge,
        signature,
    };

    let response = client
        .confirm_withdrawal(request)
        .await
        .map_err(|e| e.to_string())?;

    let response = response.into_inner();
    println!("Withdrawal confirmed successfully");
    println!("Success: {}", response.success);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Propose {
            amount,
            address_to,
            public_key,
            endpoint,
        } => {
            propose_withdrawal(endpoint, amount, address_to, public_key).await?;
        }
        Commands::Confirm {
            challenge,
            signature,
            endpoint,
        } => {
            confirm_withdrawal(endpoint, challenge, signature).await?;
        }
    }

    Ok(())
}
