use boltz_client::PublicKey;
use boltz_client::boltz::{
    BOLTZ_MAINNET_URL_V2, BoltzApiClientV2, ChainSwapDetails, Leaf, SwapTree,
};
use boltz_client::error::Error;
use boltz_client::fees::Fee;
use boltz_client::network::electrum::{ElectrumBitcoinClient, ElectrumLiquidClient};
use boltz_client::network::{BitcoinChain, Chain, LiquidChain};
use boltz_client::swaps::BtcLikeTransaction;
use boltz_client::swaps::{ChainClient, SwapScript, SwapTransactionParams, TransactionOptions};
use boltz_client::util::secrets::{Preimage, SwapKey};
use clap::Parser;
use dialoguer::Confirm;
use serde::Deserialize;
use std::fs;
use std::str::FromStr;
use std::time::Duration;

async fn refund_rescue(
    mnemonic: &str,
    claim_leaf_output: &str,
    claim_leaf_version: u8,
    refund_leaf_output: &str,
    refund_leaf_version: u8,
    blinding_key: Option<&str>,
    timeout_block_height: u32,
    server_public_key: &str,
    lockup_address: &str,
    amount: u64,
    swap_id: &str,
    refund_address: &str,
    passphrase: &str,
    swap_index: u64,
    from_network: Chain,
    to_network: Chain,
) -> Result<(BtcLikeTransaction, ChainClient, u64, String, Chain), Error> {
    let from_chain = from_network;

    let swap_tree = SwapTree {
        claim_leaf: Leaf {
            output: claim_leaf_output.to_string(),
            version: claim_leaf_version,
        },
        refund_leaf: Leaf {
            output: refund_leaf_output.to_string(),
            version: refund_leaf_version,
        },
    };

    let server_public_key = PublicKey::from_str(server_public_key)?;

    let refund_swap_key =
        SwapKey::from_chain_account(mnemonic, passphrase, from_chain, swap_index)?;

    let refund_public_key = PublicKey {
        compressed: true,
        inner: refund_swap_key.keypair.public_key(),
    };

    let lockup_details = ChainSwapDetails {
        swap_tree: swap_tree.clone(),
        lockup_address: lockup_address.to_string(),
        server_public_key,
        timeout_block_height,
        amount,
        blinding_key: blinding_key.map(|k| k.to_string()),
        refund_address: None,
        claim_address: None,
        bip21: None,
    };

    let lockup_script = SwapScript::chain_from_swap_resp(
        from_chain,
        boltz_client::boltz::Side::Lockup,
        lockup_details,
        refund_public_key,
    )?;

    println!("Lockup script created: {:?}", lockup_script);

    let mut chain_client = ChainClient::new();

    match from_network {
        Chain::Bitcoin(bitcoin_chain) => {
            chain_client =
                chain_client.with_bitcoin(ElectrumBitcoinClient::default(bitcoin_chain, None)?);
        }
        Chain::Liquid(liquid_chain) => {
            chain_client =
                chain_client.with_liquid(ElectrumLiquidClient::default(liquid_chain, None)?);
        }
    }

    match to_network {
        Chain::Bitcoin(bitcoin_chain) => {
            if !matches!(from_network, Chain::Bitcoin(_)) {
                chain_client =
                    chain_client.with_bitcoin(ElectrumBitcoinClient::default(bitcoin_chain, None)?);
            }
        }
        Chain::Liquid(liquid_chain) => {
            if !matches!(from_network, Chain::Liquid(_)) {
                chain_client =
                    chain_client.with_liquid(ElectrumLiquidClient::default(liquid_chain, None)?);
            }
        }
    }

    let boltz_api = BoltzApiClientV2::new(
        BOLTZ_MAINNET_URL_V2.to_string(),
        Some(Duration::from_secs(30)),
    );

    println!("\n=== Constructing Refund Transaction ===");
    println!("Swap ID: {}", swap_id);
    println!("Refund address: {}", refund_address);
    println!("Direction: {:?} -> {:?}", from_network, to_network);

    let fee = match from_network {
        Chain::Liquid(_) => Fee::Absolute(30),
        Chain::Bitcoin(_) => Fee::Absolute(300),
    };

    let swap_params = SwapTransactionParams {
        keys: refund_swap_key.keypair,
        output_address: refund_address.to_string(),
        fee,
        swap_id: swap_id.to_string(),
        options: Some(TransactionOptions::default().with_cooperative(true)),
        chain_client: &chain_client,
        boltz_client: &boltz_api,
    };

    println!("\nConstructing refund transaction...");
    let tx = lockup_script.construct_refund(swap_params).await?;

    println!("Refund transaction constructed successfully!");

    Ok((
        tx,
        chain_client,
        amount,
        refund_address.to_string(),
        from_network,
    ))
}

async fn claim_rescue(
    mnemonic: &str,
    claim_leaf_output: &str,
    claim_leaf_version: u8,
    refund_leaf_output: &str,
    refund_leaf_version: u8,
    blinding_key: Option<&str>,
    timeout_block_height: u32,
    server_public_key: &str,
    user_lockup_address: &str,
    server_lockup_address: &str,
    amount: u64,
    swap_id: &str,
    claim_address: &str,
    preimage: Option<&str>,
    passphrase: &str,
    swap_index: u64,
    from_network: Chain,
    to_network: Chain,
) -> Result<(BtcLikeTransaction, ChainClient, u64, String, Chain), Error> {
    let swap_tree = SwapTree {
        claim_leaf: Leaf {
            output: claim_leaf_output.to_string(),
            version: claim_leaf_version,
        },
        refund_leaf: Leaf {
            output: refund_leaf_output.to_string(),
            version: refund_leaf_version,
        },
    };

    let server_public_key = PublicKey::from_str(server_public_key)?;

    let refund_swap_key =
        SwapKey::from_chain_account(mnemonic, passphrase, from_network, swap_index)?;

    let refund_public_key = PublicKey {
        compressed: true,
        inner: refund_swap_key.keypair.public_key(),
    };

    let lockup_details = ChainSwapDetails {
        swap_tree: swap_tree.clone(),
        lockup_address: user_lockup_address.to_string(),
        server_public_key,
        timeout_block_height,
        amount,
        blinding_key: blinding_key.map(|k| k.to_string()),
        refund_address: None,
        claim_address: None,
        bip21: None,
    };

    let lockup_script = SwapScript::chain_from_swap_resp(
        from_network,
        boltz_client::boltz::Side::Lockup,
        lockup_details,
        refund_public_key,
    )?;

    let claim_swap_key = SwapKey::from_chain_account(mnemonic, passphrase, to_network, swap_index)?;

    let claim_public_key = PublicKey {
        compressed: true,
        inner: claim_swap_key.keypair.public_key(),
    };

    let claim_details = ChainSwapDetails {
        swap_tree: swap_tree.clone(),
        lockup_address: server_lockup_address.to_string(),
        server_public_key,
        timeout_block_height,
        amount,
        blinding_key: blinding_key.map(|k| k.to_string()),
        refund_address: None,
        claim_address: None,
        bip21: None,
    };

    let claim_script = SwapScript::chain_from_swap_resp(
        to_network,
        boltz_client::boltz::Side::Claim,
        claim_details,
        claim_public_key,
    )?;

    println!("Claim script created: {:?}", claim_script);

    let mut chain_client = ChainClient::new();

    match from_network {
        Chain::Bitcoin(bitcoin_chain) => {
            chain_client =
                chain_client.with_bitcoin(ElectrumBitcoinClient::default(bitcoin_chain, None)?);
        }
        Chain::Liquid(liquid_chain) => {
            chain_client =
                chain_client.with_liquid(ElectrumLiquidClient::default(liquid_chain, None)?);
        }
    }

    match to_network {
        Chain::Bitcoin(bitcoin_chain) => {
            if !matches!(from_network, Chain::Bitcoin(_)) {
                chain_client =
                    chain_client.with_bitcoin(ElectrumBitcoinClient::default(bitcoin_chain, None)?);
            }
        }
        Chain::Liquid(liquid_chain) => {
            if !matches!(from_network, Chain::Liquid(_)) {
                chain_client =
                    chain_client.with_liquid(ElectrumLiquidClient::default(liquid_chain, None)?);
            }
        }
    }

    let boltz_api = BoltzApiClientV2::new(
        BOLTZ_MAINNET_URL_V2.to_string(),
        Some(Duration::from_secs(30)),
    );

    println!("\n=== Constructing Claim Transaction ===");
    println!("Swap ID: {}", swap_id);
    println!("Claim address: {}", claim_address);
    println!("Direction: {:?} -> {:?}", from_network, to_network);

    let fee = match to_network {
        Chain::Liquid(_) => Fee::Absolute(30),
        Chain::Bitcoin(_) => Fee::Absolute(300),
    };

    let preimage = if let Some(preimage_str) = preimage {
        Preimage::from_str(preimage_str)?
    } else {
        println!(
            "⚠️  No preimage provided, but cooperative claim will use keypath spending (preimage not needed in witness)"
        );
        Preimage::from_vec(vec![0u8; 32])?
    };

    let swap_params = SwapTransactionParams {
        keys: claim_swap_key.keypair,
        output_address: claim_address.to_string(),
        fee,
        swap_id: swap_id.to_string(),
        options: Some(
            TransactionOptions::default()
                .with_cooperative(true)
                .with_chain_claim(refund_swap_key.keypair, lockup_script),
        ),
        chain_client: &chain_client,
        boltz_client: &boltz_api,
    };

    println!("\nConstructing claim transaction...");
    let tx = claim_script.construct_claim(&preimage, swap_params).await?;

    println!("Claim transaction constructed successfully!");

    Ok((
        tx,
        chain_client,
        amount,
        claim_address.to_string(),
        to_network,
    ))
}

#[derive(Parser)]
#[command(name = "recover_swap")]
#[command(about = "Recover swap claim or refund transaction", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "config.yaml")]
    config: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    user_input: UserInput,
    provider_input: ProviderInput,
}

#[derive(Debug, Deserialize)]
struct UserInput {
    mnemonic: String,
    #[serde(default)]
    passphrase: String,
    #[serde(default)]
    swap_index: u64,
    return_address: String,
}

#[derive(Debug, Deserialize)]
struct ProviderInput {
    rescue_type: String,
    claim_leaf: LeafConfig,
    refund_leaf: LeafConfig,
    #[serde(rename = "from_network")]
    from_network_str: String,
    #[serde(rename = "to_network")]
    to_network_str: String,
    blinding_key: Option<String>,
    timeout_block_height: u32,
    server_public_key: String,
    user_lockup_address: String,
    server_lockup_address: String,
    amount: u64,
    swap_id: String,
    preimage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LeafConfig {
    output: String,
    version: u8,
}

fn parse_chain(input: &str) -> Result<Chain, String> {
    match input.to_lowercase().as_str() {
        "bitcoin" | "btc" => Ok(Chain::Bitcoin(BitcoinChain::Bitcoin)),
        "liquid" | "lbtc" => Ok(Chain::Liquid(LiquidChain::Liquid)),
        _ => Err(format!(
            "Invalid chain: {}. Use 'bitcoin' or 'liquid'",
            input
        )),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!("=== Swap Rescue Tool ===\n");
    println!("Reading config from: {}\n", cli.config);

    let config_content = fs::read_to_string(&cli.config).unwrap_or_else(|e| {
        eprintln!("Error reading config file {}: {}", cli.config, e);
        std::process::exit(1);
    });

    let config: Config = serde_yaml::from_str(&config_content).unwrap_or_else(|e| {
        eprintln!("Error parsing YAML config: {}", e);
        std::process::exit(1);
    });

    let from_network = parse_chain(&config.provider_input.from_network_str).unwrap_or_else(|e| {
        eprintln!("Error parsing from_network: {}", e);
        std::process::exit(1);
    });

    let to_network = parse_chain(&config.provider_input.to_network_str).unwrap_or_else(|e| {
        eprintln!("Error parsing to_network: {}", e);
        std::process::exit(1);
    });

    let rescue_type = config.provider_input.rescue_type.to_lowercase();
    if rescue_type != "claim" && rescue_type != "refund" {
        eprintln!(
            "Error: rescue_type must be either 'claim' or 'refund', got: {}",
            config.provider_input.rescue_type
        );
        std::process::exit(1);
    }

    if rescue_type == "claim" && config.provider_input.preimage.is_none() {
        eprintln!("Error: preimage is required for coop claim");
        std::process::exit(1);
    }

    let result = if rescue_type == "refund" {
        refund_rescue(
            &config.user_input.mnemonic,
            &config.provider_input.claim_leaf.output,
            config.provider_input.claim_leaf.version,
            &config.provider_input.refund_leaf.output,
            config.provider_input.refund_leaf.version,
            config.provider_input.blinding_key.as_deref(),
            config.provider_input.timeout_block_height,
            &config.provider_input.server_public_key,
            &config.provider_input.user_lockup_address,
            config.provider_input.amount,
            &config.provider_input.swap_id,
            &config.user_input.return_address,
            &config.user_input.passphrase,
            config.user_input.swap_index,
            from_network,
            to_network,
        )
        .await
    } else {
        claim_rescue(
            &config.user_input.mnemonic,
            &config.provider_input.claim_leaf.output,
            config.provider_input.claim_leaf.version,
            &config.provider_input.refund_leaf.output,
            config.provider_input.refund_leaf.version,
            config.provider_input.blinding_key.as_deref(),
            config.provider_input.timeout_block_height,
            &config.provider_input.server_public_key,
            &config.provider_input.user_lockup_address,
            &config.provider_input.server_lockup_address,
            config.provider_input.amount,
            &config.provider_input.swap_id,
            &config.user_input.return_address,
            config.provider_input.preimage.as_deref(),
            &config.user_input.passphrase,
            config.user_input.swap_index,
            from_network,
            to_network,
        )
        .await
    };

    match result {
        Ok((tx, chain_client, amount, return_address, network)) => {
            println!("\n=== Transaction Summary ===");
            println!("Amount: {} sats", amount);
            println!("Return Address: {}", return_address);
            println!("Network: {:?}", network);
            println!("Swap ID: {}", config.provider_input.swap_id);

            let confirmed = Confirm::new()
                .with_prompt("\nDo you want to broadcast this transaction?")
                .default(false)
                .interact()
                .unwrap();

            if !confirmed {
                println!("\n❌ Transaction broadcast cancelled by user.");
                std::process::exit(0);
            }

            let tx_type = if rescue_type == "refund" {
                "refund"
            } else {
                "claim"
            };
            println!("\nBroadcasting {} transaction...", tx_type);
            match chain_client.broadcast_tx(&tx).await {
                Ok(_) => {
                    println!(
                        "\n=== {} Transaction Broadcast Successfully ===",
                        tx_type.to_uppercase()
                    );
                    println!("Transaction has been broadcast to the network");
                    println!(
                        "\n✅ {} transaction completed successfully!",
                        tx_type.to_uppercase()
                    );
                }
                Err(e) => {
                    eprintln!("\n❌ Error broadcasting transaction: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("\n❌ Error: {:?}", e);
            std::process::exit(1);
        }
    }
}
