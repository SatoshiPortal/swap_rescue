use boltz_client::PublicKey;
use boltz_client::boltz::{
    BOLTZ_MAINNET_URL_V2, BoltzApiClientV2, ChainSwapDetails, CreateSubmarineResponse, Leaf, SwapTree,
};
use boltz_client::error::Error;
use boltz_client::fees::Fee;
use boltz_client::network::electrum::{ElectrumBitcoinClient, ElectrumLiquidClient};
use boltz_client::network::{BitcoinChain, Chain, LiquidChain};
use boltz_client::swaps::BtcLikeTransaction;
use boltz_client::swaps::bitcoin::{BtcSwapScript, BtcSwapTx};
use boltz_client::swaps::liquid::{LBtcSwapScript, LBtcSwapTx};
use boltz_client::swaps::{ChainClient, SwapScript, SwapTransactionParams, TransactionOptions};
use boltz_client::util::secrets::{Preimage, SwapKey};
use std::str::FromStr;
use std::time::Duration;

pub mod gui;

pub async fn refund_rescue(
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
            chain_client = chain_client.with_liquid(ElectrumLiquidClient::new(
                liquid_chain,
                "les.bullbitcoin.com:995",
                true,  // use SSL
                true,  // validate domain (required by BullBitcoin)
                60,    // timeout in seconds
            )?);
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
                    chain_client.with_liquid(ElectrumLiquidClient::new(
                        liquid_chain,
                        "les.bullbitcoin.com:995",
                        true,  // use SSL
                        true,  // validate domain (required by BullBitcoin)
                        60,    // timeout in seconds
                    )?);
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

pub async fn claim_rescue(
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
                chain_client.with_liquid(ElectrumLiquidClient::new(
                    liquid_chain,
                    "les.bullbitcoin.com:995",
                    true,  // use SSL
                    true,  // validate domain (required by BullBitcoin)
                    60,    // timeout in seconds
                )?);
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
                    chain_client.with_liquid(ElectrumLiquidClient::new(
                        liquid_chain,
                        "les.bullbitcoin.com:995",
                        true,  // use SSL
                        true,  // validate domain (required by BullBitcoin)
                        60,    // timeout in seconds
                    )?);
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

pub async fn submarine_refund_rescue(
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
    network: Chain,
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
        SwapKey::from_submarine_account(mnemonic, passphrase, network, swap_index)?;

    let refund_public_key = PublicKey {
        compressed: true,
        inner: refund_swap_key.keypair.public_key(),
    };

    // Construct CreateSubmarineResponse for submarine swaps
    let submarine_response = CreateSubmarineResponse {
        accept_zero_conf: false,
        address: lockup_address.to_string(),
        bip21: String::new(), // Not needed for refund
        claim_public_key: server_public_key.clone(),
        expected_amount: amount,
        id: swap_id.to_string(),
        referral_id: None,
        swap_tree: swap_tree.clone(),
        timeout_block_height: timeout_block_height as u64,
        blinding_key: blinding_key.map(|k| k.to_string()),
    };

    let mut chain_client = ChainClient::new();

    match network {
        Chain::Bitcoin(bitcoin_chain) => {
            chain_client =
                chain_client.with_bitcoin(ElectrumBitcoinClient::default(bitcoin_chain, None)?);
        }
        Chain::Liquid(liquid_chain) => {
            chain_client =
                chain_client.with_liquid(ElectrumLiquidClient::new(
                    liquid_chain,
                    "les.bullbitcoin.com:995",
                    true,  // use SSL
                    true,  // validate domain (required by BullBitcoin)
                    60,    // timeout in seconds
                )?);
        }
    }

    let boltz_api = BoltzApiClientV2::new(
        BOLTZ_MAINNET_URL_V2.to_string(),
        Some(Duration::from_secs(30)),
    );

    println!("\n=== Constructing Submarine Refund Transaction ===");
    println!("Swap ID: {}", swap_id);
    println!("Refund address: {}", refund_address);
    println!("Network: {:?}", network);

    let fee = match network {
        Chain::Liquid(_) => Fee::Absolute(30),
        Chain::Bitcoin(_) => Fee::Absolute(300),
    };

    println!("\nConstructing submarine refund transaction...");

    let tx = match network {
        Chain::Liquid(_) => {
            let lockup_script = LBtcSwapScript::submarine_from_swap_resp(
                &submarine_response,
                refund_public_key,
            )?;

            println!("Lockup script created (submarine Liquid): {:?}", lockup_script);

            let liquid_client = chain_client.liquid_client()
                .ok_or_else(|| Error::Protocol("Liquid client not initialized".to_string()))?;

            let swap_tx = LBtcSwapTx::new_refund(
                lockup_script,
                refund_address,
                liquid_client,
                &boltz_api,
                swap_id.to_string(),
            ).await?;

            let signed_tx = swap_tx.sign_refund(&refund_swap_key.keypair, fee, None, true).await?;
            BtcLikeTransaction::liquid(signed_tx)
        }
        Chain::Bitcoin(_) => {
            let lockup_script = BtcSwapScript::submarine_from_swap_resp(
                &submarine_response,
                refund_public_key,
            )?;

            println!("Lockup script created (submarine Bitcoin): {:?}", lockup_script);

            let bitcoin_client = chain_client.bitcoin_client()
                .ok_or_else(|| Error::Protocol("Bitcoin client not initialized".to_string()))?;

            let swap_tx = BtcSwapTx::new_refund(
                lockup_script,
                refund_address,
                bitcoin_client,
                &boltz_api,
                swap_id.to_string(),
            ).await?;

            let signed_tx = swap_tx.sign_refund(&refund_swap_key.keypair, fee, None).await?;
            BtcLikeTransaction::bitcoin(signed_tx)
        }
    };

    println!("Submarine refund transaction constructed successfully!");

    Ok((
        tx,
        chain_client,
        amount,
        refund_address.to_string(),
        network,
    ))
}

pub fn parse_chain(input: &str) -> Result<Chain, String> {
    match input.to_lowercase().as_str() {
        "bitcoin" | "btc" => Ok(Chain::Bitcoin(BitcoinChain::Bitcoin)),
        "liquid" | "lbtc" => Ok(Chain::Liquid(LiquidChain::Liquid)),
        _ => Err(format!(
            "Invalid chain: {}. Use 'bitcoin' or 'liquid'",
            input
        )),
    }
}
