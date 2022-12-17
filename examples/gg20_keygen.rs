use anyhow::{anyhow, Context, Result};
use curv::elliptic::curves::Point;
use curv::{arithmetic::Converter, BigInt};
use futures::StreamExt;
use multi_party_ecdsa::utilities::bip32;
use std::path::PathBuf;
use structopt::StructOpt;

use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::Keygen;
use round_based::async_runtime::AsyncProtocol;

mod gg20_sm_client;
use gg20_sm_client::join_computation;
mod common;
use curv::elliptic::curves::{secp256_k1::Secp256k1, Scalar};
use opentelemetry::global;

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(short, long, default_value = "http://localhost:8000/")]
    address: surf::Url,
    #[structopt(short, long, default_value = "default-keygen")]
    room: String,
    #[structopt(short, long)]
    output: PathBuf,

    #[structopt(short, long)]
    index: u16,
    #[structopt(short, long)]
    threshold: u16,
    #[structopt(short, long)]
    number_of_parties: u16,
    #[structopt(short, long)]
    derivation_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Cli = Cli::from_args();
    let mut output_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(args.output)
        .await
        .context("cannot create output file")?;
    let (_i, incoming, outgoing) = join_computation(args.address, &args.room)
        .await
        .context("join computation")?;

    let incoming = incoming.fuse();
    tokio::pin!(incoming);
    tokio::pin!(outgoing);

    // add bip32 derive extended private key
    // derive bip32 path
    let base_u = Scalar::<Secp256k1>::random();
    let base_y = Point::<Secp256k1>::generator() * &base_u;

    let (ge, fe) = if let Some(path) = args.derivation_path {
        let path_vector: Vec<BigInt> = path
            .split('/')
            .map(|s| BigInt::from_str_radix(s.trim(), 10).unwrap())
            .collect();
        bip32::get_hd_key(base_y, path_vector)
    } else {
        (base_y, base_u)
    };

    let keygen = Keygen::new(ge, fe, args.index, args.threshold, args.number_of_parties)?;

    let output = AsyncProtocol::new(keygen, incoming, outgoing)
        .run()
        .await
        .map_err(|e| anyhow!("protocol execution terminated with error: {}", e))?;
    let output = serde_json::to_vec_pretty(&output).context("serialize output")?;

    tokio::io::copy(&mut output.as_slice(), &mut output_file)
        .await
        .context("save output to file")?;

    global::shutdown_tracer_provider();
    Ok(())
}
