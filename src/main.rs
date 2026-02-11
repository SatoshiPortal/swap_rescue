use clap::Parser;
use dialoguer::Confirm;
use serde::Deserialize;
use std::fs;
use swap_rescue::{claim_rescue, parse_chain, refund_rescue, submarine_refund_rescue};

#[derive(Parser)]
#[command(name = "swap_rescue")]
#[command(about = "Recover swap claim or refund transaction", long_about = None)]
struct Cli {
    #[arg(short, long)]
    config: Option<String>,

    #[arg(long, help = "Launch GUI mode")]
    gui: bool,
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
    #[serde(default = "default_swap_type")]
    swap_type: String,
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
    server_lockup_address: Option<String>,
    amount: u64,
    swap_id: String,
    preimage: Option<String>,
}

fn default_swap_type() -> String {
    "chain".to_string()
}

#[derive(Debug, Deserialize)]
struct LeafConfig {
    output: String,
    version: u8,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // If GUI flag is set or no config provided, launch GUI
    if cli.gui || cli.config.is_none() {
        launch_gui();
        return;
    }

    // Otherwise run CLI mode
    run_cli(cli.config.unwrap()).await;
}

fn launch_gui() {
    use iced::{application, window, Font, Settings, Size};
    use swap_rescue::gui::SwapRescueApp;

    let settings = Settings {
        default_font: Font::with_name("Golos Text"),
        ..Default::default()
    };

    let _ = application(SwapRescueApp::title, SwapRescueApp::update, SwapRescueApp::view)
        .settings(settings)
        .window(window::Settings {
            size: Size::new(800.0, 900.0),
            min_size: Some(Size::new(600.0, 700.0)),
            ..Default::default()
        })
        .font(include_bytes!("../assets/GolosText-Regular.ttf"))
        .run_with(SwapRescueApp::new);
}

async fn run_cli(config_path: String) {
    println!("=== Swap Rescue Tool ===\n");
    println!("Reading config from: {}\n", config_path);

    let config_content = fs::read_to_string(&config_path).unwrap_or_else(|e| {
        eprintln!("Error reading config file {}: {}", config_path, e);
        std::process::exit(1);
    });

    let config: Config = serde_yaml::from_str(&config_content).unwrap_or_else(|e| {
        eprintln!("Error parsing YAML config: {}", e);
        std::process::exit(1);
    });

    // Validate config
    if let Err(e) = validate_cli_config(&config) {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    let from_network = parse_chain(&config.provider_input.from_network_str).unwrap_or_else(|e| {
        eprintln!("Error parsing from_network: {}", e);
        std::process::exit(1);
    });

    let rescue_type = config.provider_input.rescue_type.to_lowercase();
    let swap_type = config.provider_input.swap_type.to_lowercase();

    let result = if rescue_type == "refund" {
        if swap_type == "submarine" {
            // Submarine refund - only uses from_network
            submarine_refund_rescue(
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
            )
            .await
        } else {
            // Chain refund
            let to_network = parse_chain(&config.provider_input.to_network_str).unwrap_or_else(|e| {
                eprintln!("Error parsing to_network: {}", e);
                std::process::exit(1);
            });

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
        }
    } else {
        // Claim rescue (only for chain swaps)
        let to_network = parse_chain(&config.provider_input.to_network_str).unwrap_or_else(|e| {
            eprintln!("Error parsing to_network: {}", e);
            std::process::exit(1);
        });

        let server_lockup_address = config.provider_input.server_lockup_address
            .as_deref()
            .unwrap_or_else(|| {
                eprintln!("Error: server_lockup_address is required for claim rescue");
                std::process::exit(1);
            });

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
            server_lockup_address,
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

fn validate_cli_config(config: &Config) -> Result<(), String> {
    // Validate user_input
    if config.user_input.mnemonic.trim().is_empty() {
        return Err("Error: mnemonic is required in user_input".to_string());
    }

    if config.user_input.return_address.trim().is_empty() {
        return Err("Error: return_address is required in user_input".to_string());
    }

    // Validate provider_input
    if config.provider_input.rescue_type.trim().is_empty() {
        return Err("Error: rescue_type is required in provider_input".to_string());
    }

    let rescue_type = config.provider_input.rescue_type.to_lowercase();
    if rescue_type != "claim" && rescue_type != "refund" {
        return Err(format!(
            "Error: rescue_type must be 'claim' or 'refund', got '{}'",
            config.provider_input.rescue_type
        ));
    }

    let swap_type = config.provider_input.swap_type.to_lowercase();
    if swap_type != "chain" && swap_type != "submarine" {
        return Err(format!(
            "Error: swap_type must be 'chain' or 'submarine', got '{}'",
            config.provider_input.swap_type
        ));
    }

    if config.provider_input.claim_leaf.output.trim().is_empty() {
        return Err("Error: claim_leaf.output is required in provider_input".to_string());
    }

    if config.provider_input.refund_leaf.output.trim().is_empty() {
        return Err("Error: refund_leaf.output is required in provider_input".to_string());
    }

    if config.provider_input.from_network_str.trim().is_empty() {
        return Err("Error: from_network is required in provider_input".to_string());
    }

    // For submarine swaps, to_network may not be needed
    if swap_type == "chain" && config.provider_input.to_network_str.trim().is_empty() {
        return Err("Error: to_network is required in provider_input for chain swaps".to_string());
    }

    if config.provider_input.timeout_block_height == 0 {
        return Err("Error: timeout_block_height must be greater than 0 in provider_input".to_string());
    }

    if config.provider_input.server_public_key.trim().is_empty() {
        return Err("Error: server_public_key is required in provider_input".to_string());
    }

    if config.provider_input.user_lockup_address.trim().is_empty() {
        return Err("Error: user_lockup_address is required in provider_input".to_string());
    }

    // server_lockup_address is only required for chain swaps with claim rescue
    if swap_type == "chain" && rescue_type == "claim" {
        if let Some(addr) = &config.provider_input.server_lockup_address {
            if addr.trim().is_empty() {
                return Err("Error: server_lockup_address cannot be empty for chain swap claims".to_string());
            }
        } else {
            return Err("Error: server_lockup_address is required for chain swap claims".to_string());
        }
    }

    if config.provider_input.amount == 0 {
        return Err("Error: amount must be greater than 0 in provider_input".to_string());
    }

    if config.provider_input.swap_id.trim().is_empty() {
        return Err("Error: swap_id is required in provider_input".to_string());
    }

    // Validate that preimage is provided if rescue_type is "claim" and swap_type is "chain"
    if rescue_type == "claim" && swap_type == "chain" && config.provider_input.preimage.is_none() {
        return Err("Error: preimage is required in provider_input when rescue_type is 'claim' for chain swaps".to_string());
    }

    if let Some(preimage) = &config.provider_input.preimage {
        if preimage.trim().is_empty() {
            return Err("Error: preimage cannot be empty when provided".to_string());
        }
    }

    // Passphrase is optional - no validation needed

    Ok(())
}
