// Simple querie examples:
// "0x1::Account::Balance<0x1::XFI::T>",
// "0x1::Account::Balance<0x1::Coins::ETH>",
// "0x1::Account::Balance<0x1::Coins::BTC>",
// "0x1::Account::Balance<0x1::Coins::USDT>",
// "0x1::Account::Balance<0x1::Coins::SXFI>",

#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::path::{Path, PathBuf};
use anyhow::{Result, Error, anyhow};
use http::Uri;
use clap::Clap;
use libra::prelude::*;
use lang::compiler::bech32::{bech32_into_libra, HRP};
use dnclient::blocking as net;

mod ser;
mod tte;

const VERSION: &str = git_hash::crate_version_with_git_hash_short!();
const JSON_SCHEMA_STDOUT: &str = "-";

#[derive(Clap, Debug)]
#[clap(name = "Move resource viewer", version = VERSION)]
struct Cfg {
    /// Owner's address
    #[clap(long, short)]
    address: String,

    /// Query in `TypeTag` format,
    /// one-line address+type description.
    /// Mainly, in most cases should be StructTag.
    /// Additionaly can contain index at the end.
    /// Query examples:
    /// "0x1::Account::Balance<0x1::XFI::T>",
    /// "0x1::Account::Balance<0x1::Coins::ETH>"
    #[clap(long, short)]
    query: tte::TypeTagQuery,

    /// Time: maximum block number
    #[clap(long, short)]
    height: Option<u128>,

    /// Output file path
    #[clap(long, short)]
    output: PathBuf,

    /// Sets output format to JSON.
    /// Optional, `true` if output file extension is .json
    #[clap(long, short)]
    json: Option<bool>,

    /// Node REST API address
    #[clap(long)]
    api: Uri,

    /// Enables compatibility mode
    #[clap(long, short)]
    compat: bool,

    /// Export JSON schema for output format.
    /// Special value for write to stdout: "-"
    #[clap(long = "json-schema")]
    json_schema: Option<PathBuf>,
}

fn main() -> Result<(), Error> {
    init_logger()
        .map_err(|err| eprintln!("Error: {}", err))
        .ok();
    run().map_err(|err| {
        error!("{}", err);
        err
    })
}

fn init_logger() -> Result<(), impl std::error::Error> {
    use env_logger::*;

    let mut builder = Builder::from_env(Env::default());
    builder.format_timestamp(None);
    builder.try_init()
}

fn run() -> Result<(), Error> {
    let cfg = Cfg::parse();

    produce_json_schema(&cfg);

    let host = cfg.api;
    let output = cfg.output;
    let height = cfg.height;
    let json = cfg.json.unwrap_or_else(|| {
        output
            .extension()
            .map(|ext| ext == "json")
            .unwrap_or_default()
    });
    let (tte, index) = cfg.query.into_inner();
    let addr = if cfg.address.starts_with(HRP) {
        AccountAddress::from_hex_literal(&bech32_into_libra(&cfg.address)?)
    } else {
        AccountAddress::from_hex_literal(&cfg.address)
    }?;

    match tte {
        TypeTag::Struct(st) => {
            let key = ResourceKey::new(addr, st.clone());
            let res = net::get_resource(&key, &host, height);
            res.map(|resp| {
                let bytes = resp.as_bytes();
                if !bytes.is_empty() {
                    let client = net::client::DnodeRestClient::new(host, height);

                    // Internally produce FatStructType (with layout) for StructTag by
                    // resolving & de-.. entire deps-chain.
                    let annotator = rv::MoveValueAnnotator::new_no_stdlib(&client);

                    annotator
                        .view_resource(&st, &bytes)
                        .and_then(|result| {
                            let height = resp.block();
                            if json {
                                serde_json::ser::to_string_pretty(
                                    &ser::AnnotatedMoveStructWrapper { height, result },
                                )
                                .map_err(|err| anyhow!("{}", err))
                            } else {
                                Ok(format!("{}", result))
                            }
                        })
                        .map(|result| write_output(&output, result))
                } else {
                    Err(anyhow!("Resource not found, result is empty"))
                }
            })
            .and_then(|result| result)
        }

        TypeTag::Vector(tt) => Err(anyhow!(
            "Unsupported root type Vec<{}>{:?}",
            tt,
            index.map(|v| [v]).unwrap_or_default()
        )),

        _ => Err(anyhow!("Unsupported type {}", tte)),
    }
}

fn produce_json_schema(cfg: &Cfg) {
    if let Some(path) = cfg.json_schema.as_ref() {
        let schema = ser::produce_json_schema();
        let render = serde_json::to_string_pretty(&schema).unwrap();
        if path.as_os_str() == JSON_SCHEMA_STDOUT {
            println!("{}", &render);
        } else {
            write_output(&path, render);
            info!("schema generated successfully");
        }
    }
}

fn write_output(path: &Path, result: String) {
    use std::io::prelude::*;
    std::fs::File::create(path)
        .and_then(|mut f| f.write_all(result.as_bytes()))
        .map_err(|err| error!("Cannot write output: {}", err))
        .ok();
}
