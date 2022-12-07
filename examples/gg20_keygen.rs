use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::keygen::Keygen;
use round_based::async_runtime::AsyncProtocol;
use std::env::remove_var;
use std::env::var;
use std::env::vars;
use std::path::PathBuf;
use std::str::FromStr;
use std::vec;
use structopt::StructOpt;
use tonic::{
    metadata::{MetadataKey, MetadataMap},
    transport::ClientTlsConfig,
};
use url::Url;
mod gg20_sm_client;
use gg20_sm_client::join_computation;
mod common;
use opentelemetry::global;
use opentelemetry::sdk::trace as sdktrace;
use opentelemetry::trace::{FutureExt, TraceError};
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry::Key;

use opentelemetry::{Context as o_ctx, KeyValue};
use opentelemetry_otlp::{ExportConfig, WithExportConfig};

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
}

const ENDPOINT: &str = "OTLP_TONIC_ENDPOINT";
const HEADER_PREFIX: &str = "OTLP_TONIC_";
fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    let endpoint = var(ENDPOINT).unwrap_or_else(|_| {
        panic!(
            "You must specify and endpoint to connect to with the variable {:?}.",
            ENDPOINT
        )
    });
    let endpoint = Url::parse(&endpoint).expect("endpoint is not a valid url");
    remove_var(ENDPOINT);

    let mut metadata = MetadataMap::new();
    for (key, value) in vars()
        .filter(|(name, _)| name.starts_with(HEADER_PREFIX))
        .map(|(name, value)| {
            let header_name = name
                .strip_prefix(HEADER_PREFIX)
                .map(|h| h.replace('_', "-"))
                .map(|h| h.to_ascii_lowercase())
                .unwrap();
            (header_name, value)
        })
    {
        metadata.insert(MetadataKey::from_str(&key).unwrap(), value.parse().unwrap());
    }

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint.as_str())
                .with_metadata(metadata)
                .with_tls_config(
                    ClientTlsConfig::new().domain_name(
                        endpoint
                            .host_str()
                            .expect("the specified endpoint should have a valid host"),
                    ),
                ),
        )
        .install_batch(opentelemetry::runtime::Tokio)
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

    // trace setup
    let tracer = init_tracer()?;

    let span = tracer.start("root");
    let cx = o_ctx::current_with_span(span);

    let s_span = cx.span();

    s_span.add_event("keygen staring".to_string(), vec![]);

    s_span.add_event(
        "cli-properties".to_string(),
        vec![
            Key::new("index").i64(args.index as i64),
            Key::new("threshold").i64(args.threshold as i64),
            Key::new("number_of_parties").i64(args.number_of_parties as i64),
        ],
    );

    let keygen = Keygen::new(args.index, args.threshold, args.number_of_parties)?;

    let output = AsyncProtocol::new(keygen, incoming, outgoing)
        .run()
        .with_context(cx.clone())
        .await
        .map_err(|e| anyhow!("protocol execution terminated with error: {}", e))?;
    let output = serde_json::to_vec_pretty(&output).context("serialize output")?;

    tokio::io::copy(&mut output.as_slice(), &mut output_file)
        .with_context(cx.clone())
        .await
        .context("save output to file")?;

    global::shutdown_tracer_provider();

    Ok(())
}
