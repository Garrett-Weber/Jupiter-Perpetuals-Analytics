use std::collections::HashMap;
use std::fs;
use std::ops::{Add as _, Div as _, Mul as _, Sub as _};
use std::str::FromStr;
use std::time::SystemTime;

use anchor_lang::{AccountDeserialize as _, AnchorDeserialize as _, Discriminator as _};
use base64::Engine;
use clap::Parser;
use pyth_sdk_solana::load_price_feed_from_account;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use thousands::Separable;

const _PERPETUALS_PUBKEY: &str = "H4ND9aYttUVLFmNypZqLjZ52FYiGvdEB45GmwNoKEjTj";
const _FUNDED_PUBKEY: &str = "HVSZJ2juJnMxd6yCNarTL56YmgUqzfUiwM7y7LtTXKHR";

fn get_price_from_pyth_account(
    connection: &RpcClient,
    pyth_account_pubkey: &Pubkey,
) -> Result<f64, Box<dyn std::error::Error>> {
    let mut pyth_account = connection.get_account(pyth_account_pubkey)?;
    let price_feed = load_price_feed_from_account(&pyth_account_pubkey, &mut pyth_account)?;
    let price = price_feed.get_price_unchecked();
    let price_as_float = (price.price as f64).div((10 as u32).pow(8) as f64);
    Ok(price_as_float)
}

fn get_program_accounts_with_discrim(
    connection: &RpcClient,
    program_address: &str,
    discrim: &[u8],
) -> Result<
    Vec<(solana_sdk::pubkey::Pubkey, solana_sdk::account::Account)>,
    Box<dyn std::error::Error>,
> {
    use solana_client::{
        rpc_config::RpcProgramAccountsConfig,
        rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
    };

    let memcmp = RpcFilterType::Memcmp(Memcmp::new(0, MemcmpEncodedBytes::Bytes(discrim.into())));
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![memcmp]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..Default::default()
        },
        ..Default::default()
    };
    let accounts = connection.get_program_accounts_with_config(
        &solana_sdk::pubkey::Pubkey::from_str(program_address)?,
        config,
    )?;

    return Ok(accounts);
}

fn _get_fees_from_position(
    connection: &RpcClient,
    position_pubkey: &Pubkey,
    position: &perp_abi::Position,
    custody: &perp_abi::Custody,
) -> Result<perp_abi::PnlAndFee, Box<dyn std::error::Error>> {
    let tx = solana_sdk::transaction::Transaction::new_unsigned(
        solana_sdk::message::Message::new_with_blockhash(
            &[solana_sdk::instruction::Instruction::new_with_bytes(
                perp_abi::ID,
                &perp_abi::instruction::GetPnl::DISCRIMINATOR,
                vec![
                    AccountMeta::new_readonly(Pubkey::from_str(_PERPETUALS_PUBKEY)?, false),
                    AccountMeta::new_readonly(position.pool, false),
                    AccountMeta::new_readonly(*position_pubkey, false),
                    AccountMeta::new_readonly(position.custody, false),
                    AccountMeta::new_readonly(custody.oracle.oracle_account, false),
                    AccountMeta::new_readonly(position.collateral_custody, false),
                ],
            )],
            Some(&Pubkey::from_str(_FUNDED_PUBKEY)?),
            &connection.get_latest_blockhash()?,
        ),
    );
    let mut data = base64::prelude::BASE64_STANDARD.decode(
        connection
            .simulate_transaction(&tx)?
            .value
            .return_data
            .unwrap_or_default()
            .data
            .0,
    )?;
    data.resize(41, 0);
    let pnl_and_fee = perp_abi::PnlAndFee::try_from_slice(&data)?;
    Ok(pnl_and_fee)
}

#[derive(Parser)]
#[command(version, about = "Collects analytics about Jup perpetuals usage")]
struct Args {
    /// Solana RPC URL
    #[arg(short)]
    rpc_url: String,
    /// Export to CSV
    #[arg(short)]
    csv_path: Option<String>,
    /// Silent
    #[arg(short)]
    silent: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut pubkey_to_custody: HashMap<Pubkey, perp_abi::state::Custody> = HashMap::new();
    let mut custody_pubkey_to_borrow_rate: HashMap<Pubkey, f64> = HashMap::new();
    let mut mint_to_price: HashMap<Pubkey, f64> = HashMap::new();

    let rpc_client = RpcClient::new(args.rpc_url);

    let pool_accounts = get_program_accounts_with_discrim(
        &rpc_client,
        &perp_abi::ID.to_string(),
        &perp_abi::state::Pool::DISCRIMINATOR,
    )?;

    let pool = perp_abi::state::Pool::try_deserialize(&mut &*pool_accounts[0].1.data)?;
    let total_pool_value: f64 = spl_token::amount_to_ui_amount(pool.aum_usd as u64, 6);

    let unix_time = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let custody_accounts = get_program_accounts_with_discrim(
        &rpc_client,
        &perp_abi::ID.to_string(),
        &perp_abi::state::Custody::DISCRIMINATOR,
    )?;

    let mut stable_custodys = vec![];
    let mut stable_aum = 0;
    let mut stable_borrow = 0;

    for (custody_pubkey, custody) in custody_accounts {
        let custody = perp_abi::state::Custody::try_deserialize(&mut &*custody.data)?;
        let price = get_price_from_pyth_account(&rpc_client, &custody.oracle.oracle_account)?;
        pubkey_to_custody.insert(custody_pubkey, custody);

        if price.round() == 1.0 {
            // stablecoin borrow rates set by utilization percentage of all stablecoins
            stable_custodys.push(custody_pubkey);
            stable_aum += custody.assets.owned;
            stable_borrow += custody.assets.locked;
        } else {
            mint_to_price.insert(custody.mint, price);
            // non-stablecoin borrow rates set by utilization percentage
            custody_pubkey_to_borrow_rate.insert(
                custody_pubkey,
                spl_token::amount_to_ui_amount(custody.assets.locked, 6)
                    .div(spl_token::amount_to_ui_amount(custody.assets.owned, 6))
                    .mul(custody.funding_rate_state.hourly_funding_bps as f64),
            );
        }
    }

    for stable_custody in stable_custodys {
        custody_pubkey_to_borrow_rate.insert(
            stable_custody,
            (stable_borrow as f64).div(stable_aum as f64),
        );
    }

    let position_accounts = get_program_accounts_with_discrim(
        &rpc_client,
        &perp_abi::ID.to_string(),
        &perp_abi::state::Position::DISCRIMINATOR,
    )?;

    let mut num_positions: u64 = 0;
    let mut num_longs: u64 = 0;
    let mut num_winning: u64 = 0;
    let mut long_short_sign: f64 = 0.0;
    let mut cumulative_positions: f64 = 0.0;
    let mut cumulative_long: f64 = 0.0;
    let mut cumulative_positions_at_entry: f64 = 0.0;
    let mut cumulative_collateral: f64 = 0.0;
    let mut cumulative_collateral_at_entry: f64 = 0.0;
    let mut cumulative_fees: f64 = 0.0;
    let mut cumulative_pnl: f64 = 0.0;

    let mut highest_unrealized_profit: f64 = 0.0;
    let mut highest_unrealized_losses: f64 = 0.0;

    let mut most_profitable_trade: (Pubkey, f64, f64, perp_abi::Side, Pubkey) = Default::default();
    let mut least_profitable_trade: (Pubkey, f64, f64, perp_abi::Side, Pubkey) = Default::default();

    for (position_pubkey, position) in position_accounts {
        let position = perp_abi::state::Position::try_deserialize(&mut &*position.data)?;
        if position.size_usd != 0 {
            num_positions += 1;

            let mint = pubkey_to_custody.get(&position.custody).unwrap().mint;
            let amount = (position.size_usd as f64).div(position.price as f64);
            let price_at_entry = spl_token::amount_to_ui_amount(position.price, 6);
            let price = mint_to_price.get(&mint).unwrap();
            let interval = (unix_time.sub(position.update_time as u64) as f64).div(3600.0);

            let current_position_value: f64 = amount.mul(price);
            let position_value_at_entry = spl_token::amount_to_ui_amount(position.size_usd, 6);

            let entry_fees: f64 = position_value_at_entry
                .mul(pool.fees.increase_position_bps as f64)
                .div(10_000.0);

            let borrow_fees: f64 = custody_pubkey_to_borrow_rate
                .get(&position.collateral_custody)
                .unwrap()
                // mul by hours
                .mul(interval)
                // get value in USD
                .mul(position_value_at_entry)
                // BPS to absolute value
                .div(10_000.0);

            if let perp_abi::Side::Long = position.side {
                num_longs += 1;
                long_short_sign = 1.0;
                cumulative_long += current_position_value;
            } else if let perp_abi::Side::Short = position.side {
                long_short_sign = -1.0;
            }

            let collateral_at_entry = spl_token::amount_to_ui_amount(position.collateral_usd, 6);
            let current_collateral: f64 =
            // get collateral at entry
            collateral_at_entry.add(
                    // add difference in value between now and entry
                    amount
                        .mul(price.sub(price_at_entry))
                        // short's price is reversed
                        .mul(long_short_sign),
                );

            cumulative_positions_at_entry += position_value_at_entry;
            cumulative_collateral_at_entry += collateral_at_entry;
            cumulative_positions += current_position_value;
            cumulative_collateral += current_collateral;

            // paper unrealized pnl
            let unrealized_pnl = current_position_value
                .sub(position_value_at_entry)
                .mul(long_short_sign);

            if unrealized_pnl > 0.0 {
                num_winning += 1;
                if unrealized_pnl > highest_unrealized_profit {
                    highest_unrealized_profit = unrealized_pnl;
                    most_profitable_trade = (
                        position_pubkey,
                        unrealized_pnl,
                        price_at_entry,
                        position.side,
                        mint,
                    );
                }
            }

            if highest_unrealized_losses > unrealized_pnl {
                highest_unrealized_losses = unrealized_pnl;
                least_profitable_trade = (
                    position_pubkey,
                    unrealized_pnl,
                    price_at_entry,
                    position.side,
                    mint,
                );
            }

            cumulative_pnl += unrealized_pnl;
            cumulative_fees += entry_fees.mul(2.0).add(borrow_fees);
        }
    }

    let average_leverage_at_entry =
        (cumulative_positions_at_entry as f64).div(cumulative_collateral_at_entry as f64);
    let average_effective_leverage =
        (cumulative_positions as f64).div(cumulative_collateral as f64);
    let num_short = num_positions.sub(num_longs);

    if !args.silent {
        // Desperately need string interpolation in rust
        let total_pool_value_str = total_pool_value.round().separate_with_commas();
        let unrealized_pnl = cumulative_pnl.round().separate_with_commas();
        let total_fees = cumulative_fees.round().separate_with_commas();
        let real_unrealized_pnl = cumulative_pnl
            .sub(cumulative_fees)
            .round()
            .separate_with_commas();
        let total_position_value = cumulative_positions.round().separate_with_commas();
        let total_collateral_value = cumulative_collateral.round().separate_with_commas();
        let value_long = cumulative_long.round().separate_with_commas();
        let value_short = cumulative_positions
            .sub(cumulative_long)
            .round()
            .separate_with_commas();
        let long_short_ratio = (num_longs as f64).div(num_positions.sub(num_longs) as f64);
        let long_short_value = cumulative_long.div(cumulative_positions.sub(cumulative_long));
        let num_losing = num_positions.sub(num_winning);
        println!(
            "Unix time: {unix_time}
Total pool value: ${total_pool_value_str}
Total traders unrealized paper P&L: ${unrealized_pnl}
Total traders fees: ${total_fees}
Total traders unrealized real P&L ${real_unrealized_pnl}
Total value of positions: ${total_position_value}
Total value of collateral: ${total_collateral_value}
Average leverage at entry: {average_leverage_at_entry:.4}
Average effective leverage: {average_effective_leverage:.4}
Long trades: {num_longs} (${value_long})
Short trades: {num_short} (${value_short})
L/S ratio: {long_short_ratio:.4} ({long_short_value:.4})
Winning trades: {num_winning} Losing trades: {num_losing}"
        );

        println!(
        "Most profitable open trade: {} Open P&L: ${} Entry Price ${:.2} Side: {:?} Mint {}\nMost unprofitable open trade: {} Open P&L: ${} Entry Price ${:.2} Side: {:?} Mint {}",
        most_profitable_trade.0,
        most_profitable_trade.1.round().separate_with_commas(),
        most_profitable_trade.2,
        most_profitable_trade.3,
        most_profitable_trade.4,
        least_profitable_trade.0,
        least_profitable_trade.1.round().separate_with_commas(),
        least_profitable_trade.2,
        least_profitable_trade.3,
        least_profitable_trade.4,
        );
    }

    // CSV exports for plotting data over time
    if let Some(csv_path) = args.csv_path {
        let csv_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(csv_path)?;
        let mut csv_writer = csv::Writer::from_writer(csv_file.try_clone()?);
        if csv_file.metadata()?.len() == 0 {
            csv_writer.write_record(&[
                "Unix Time",
                "Total Pool Value",
                "Unrealized Paper P&L",
                "Total Fees",
                "Total Value of Positions",
                "Total Value of Collateral",
                "Average Leverage At Entry",
                "Average Effective Leverage",
                "Long Trades",
                "Long Value",
                "Short Trades",
                "Short Value",
            ])?;
        }
        csv_writer.serialize((
            unix_time,
            total_pool_value,
            cumulative_pnl,
            cumulative_fees,
            cumulative_positions,
            cumulative_collateral,
            average_leverage_at_entry,
            average_effective_leverage,
            num_longs,
            cumulative_long,
            num_short,
            cumulative_positions.sub(cumulative_long),
        ))?;
        csv_writer.flush()?;
    }
    Ok(())
}
