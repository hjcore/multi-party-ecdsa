use round_based::Msg;
use sha2::Sha256;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::reshaing::refresh_message::RefreshMessage;
use anyhow::{ Context, Result};
use futures::{StreamExt, SinkExt};
use std::path::PathBuf;
use std::vec;
use structopt::StructOpt;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::{ LocalKey};

mod gg20_sm_client;
use gg20_sm_client::join_computation;
mod common;
use curv::elliptic::curves::secp256_k1::Secp256k1;
use futures::stream::{self};

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
    number_of_parties: u16,
    #[structopt(short, long)]
    local_share: PathBuf,
    #[structopt(short, long)]
    remove_party_indices: Option<Vec<u16>>,
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

    let local_share = tokio::fs::read(args.local_share)
        .await
        .context("cannot read local share")?;

    let mut local_key: LocalKey<Secp256k1> =
        serde_json::from_slice(&local_share).context("parse local share")?;

    let (_i, incoming, outgoing) =
        join_computation::<RefreshMessage<Secp256k1, Sha256, 256>>(args.address, &args.room)
            .await
            .context("join computation")?;

    let incoming = incoming.fuse();
    tokio::pin!(incoming);
    tokio::pin!(outgoing);

    let (mut msg, dk) = RefreshMessage::<Secp256k1, Sha256, 256>::distribute(
        local_key.i,
        &mut local_key,
        args.number_of_parties,
    )
    .unwrap();

    // msg.add_remove_party_indices(args.remove_party_indices.unwrap_or(vec![]));

    let mut m = vec![Msg {
        sender: local_key.i,
        receiver: None,
        body: msg.clone(),
    }];

    let mut out_msgs = stream::iter(m.drain(..).map(Ok));
    outgoing.send_all(&mut out_msgs).await.unwrap();

    let mut msgs = vec![msg];
    loop {
        if let Some(val) = incoming.next().await {
            if val.is_err() {
                continue;
            }

            let msg = val.unwrap();
            println!("got a new msg");
            println!(
                "local_party_id: {}   sender: {} remove_party_index: {:?}, old_party_index: {}",
                args.index, msg.sender, msg.body.remove_party_indices, msg.body.old_party_index
            );

            if msg.sender == args.index {
                continue;
            }

            msgs.push(msg.body);
            if msgs.len() == args.number_of_parties as usize {
                println!("recv done...........");

                msgs.sort_by(|a, b| a.old_party_index.cmp(&b.old_party_index));
                for m in msgs.clone() {
                    println!(
                        "Collect msgs------> Index: {} Msg Index: {} Remove Index:{:?} pubkey:{:?} {:?}",
                        local_key.i, m.old_party_index, m.remove_party_indices, m.public_key.x_coord(), m.public_key.y_coord()
                    )
                }

                RefreshMessage::collect(&msgs, &mut local_key, dk, &vec![])
                    .expect("collect failed!");

                println!(
                    "result pubkey {:?}-{:?}",
                    local_key.public_key().x_coord(),
                    local_key.public_key().y_coord()
                );
                let output = serde_json::to_vec_pretty(&local_key).context("serialize output")?;
                tokio::io::copy(&mut output.as_slice(), &mut output_file)
                    .await
                    .context("save output to file")?;

                break;
            }
        }
    }

    Ok(())
}
