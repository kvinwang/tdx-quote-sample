use base64::prelude::*;
use clap::Parser;
use scale::{Decode, Encode};
use serde::Deserialize;
use std::{error::Error, time::SystemTime};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// PCCS URL
    #[arg(long, default_value = "https://localhost:8081/sgx/certification/v4/")]
    pccs: String,

    /// Worker URL
    #[arg(
        long,
        default_value = "https://teleport.nanometer.dev/tee-attestations/quote"
    )]
    worker: String,

    /// Output quote file
    #[arg(long, default_value = "quote.bin")]
    quote: String,

    /// Output collateral file
    #[arg(long, default_value = "collateral.bin")]
    collateral: String,
}

#[derive(Debug, Deserialize)]
struct QuoteResponse {
    quote: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let quote_response = reqwest::get(&args.worker)
        .await?
        .json::<QuoteResponse>()
        .await?;
    println!("Quote response: {:?}", quote_response);

    let quote_bin = BASE64_STANDARD.decode(&quote_response.quote)?;
    let collateral_response = dcap_qvl::collateral::get_collateral(
        &args.pccs,
        &quote_bin,
        std::time::Duration::from_secs(10),
    )
    .await?;
    println!("Collateral response: {:?}", collateral_response);
    std::fs::write(&args.quote, &quote_bin)?;
    println!("Quote written to {}", args.quote);
    let collateral_bin = collateral_response.encode();
    std::fs::write(&args.collateral, &collateral_bin)?;
    println!("Collateral written to {}", args.collateral);
    verify_quote(&args.quote, &args.collateral)?;
    Ok(())
}

fn verify_quote(quote_file: &str, collateral_file: &str) -> Result<(), Box<dyn Error>> {
    let quote = std::fs::read(quote_file)?;
    let collateral = std::fs::read(collateral_file)?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let collateral = dcap_qvl::QuoteCollateralV3::decode(&mut &collateral[..])?;
    let _verified =
        dcap_qvl::verify::verify(&quote, &collateral, now).expect("Failed to verify quote");
    println!("Verified");
    Ok(())
}
