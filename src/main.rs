#[macro_use]
extern crate log;

#[macro_use]
extern crate anyhow;

use domain_types::DOMAIN_ID;
use serde::{Deserialize, Serialize};
use simplelog::{CombinedLogger, Config as LogConfig, LevelFilter};
use std::{env::args, path::PathBuf, sync::Arc};
use tdn::{prelude::*, types::primitives::Result};
use tdn_did::{generate_mnemonic, generate_peer, Count, Language};
use tokio::sync::{mpsc::Sender, RwLock};

mod layer;
mod models;
mod rpc;
mod storage;

const DEFAULT_PROVIDER_NAME: &'static str = "domain.esse";
const DEFAULT_PROVIDER_PROXY: bool = true;

const DEFAULT_P2P_ADDR: &'static str = "0.0.0.0:7350";
const DEFAULT_HTTP_ADDR: &'static str = "127.0.0.1:7351";
const DEFAULT_LOG_FILE: &'static str = "domain.log.txt";

/// parse custom config from config.toml.
#[derive(Serialize, Deserialize, Debug)]
pub struct CustomConfig {
    pub name: String,
    pub proxy: bool,
    pub mnemonic: String,
}

fn custom_config_str(config: &CustomConfig) -> String {
    format!(
        r#"## Domain custom Config.
## domain server name.
name = "{}"

## domain is support proxy request/response.
proxy = {}

## domain server default mnemonic words (keep PeerId same).
mnemonic = "{}"
"#,
        config.name, config.proxy, config.mnemonic
    )
}

#[tokio::main]
async fn main() {
    let db_path = args().nth(1).unwrap_or("./.tdn".to_owned());

    if std::fs::metadata(&db_path).is_err() {
        std::fs::create_dir(&db_path).unwrap();
    }

    let _ = start(db_path).await;
}

pub async fn start(db_path: String) -> Result<()> {
    let db_path = PathBuf::from(db_path);
    if !db_path.exists() {
        tokio::fs::create_dir_all(&db_path).await?;
    }

    storage::init(&db_path).await?;

    init_log(db_path.clone());
    info!("Core storage path {:?}", db_path);

    let mut config = Config::default();
    config.db_path = Some(db_path.clone());
    config.rpc_addr = DEFAULT_HTTP_ADDR.parse().unwrap();
    config.p2p_peer = Peer::socket(DEFAULT_P2P_ADDR.parse().unwrap());
    config.p2p_allowlist.append(&mut vec![Peer::socket(
        "1.15.156.199:7364".parse().unwrap(),
    )]);
    config.group_ids = vec![DOMAIN_ID];
    let config = Config::load_save(db_path.clone(), config).await?;
    let custom: Option<CustomConfig> = Config::load_custom(db_path.clone()).await;
    let CustomConfig {
        name,
        proxy,
        mnemonic,
    } = if let Some(custom) = custom {
        custom
    } else {
        let mnemonic = generate_mnemonic(Language::English, Count::Words12);
        let custom = CustomConfig {
            name: DEFAULT_PROVIDER_NAME.to_owned(),
            proxy: DEFAULT_PROVIDER_PROXY,
            mnemonic: mnemonic,
        };
        Config::append_custom(db_path.clone(), &custom_config_str(&custom)).await?;
        custom
    };

    info!("Config RPC HTTP : {:?}", config.rpc_addr);
    info!(
        "Config P2P      : {} {:?}",
        config.p2p_peer.transport.to_str(),
        config.p2p_peer.socket
    );

    let _rand_secret = config.secret.clone();
    let pkey = generate_peer(Language::English, &mnemonic, 0, 0, None)?;
    let (peer_id, sender, mut recver) = start_with_config_and_key(config, pkey).await?;
    info!("Network Peer id : {}", peer_id.to_hex());

    let layer = Arc::new(RwLock::new(
        layer::Layer::new(db_path, name, peer_id, proxy).await?,
    ));

    let rpc_handler = rpc::new_rpc_handler(layer.clone());

    while let Some(message) = recver.recv().await {
        match message {
            ReceiveMessage::Own(_o_msg) => {
                // Self distributed domain service.
            }
            ReceiveMessage::Group(_g_msg) => {
                // Other domain services.
            }
            ReceiveMessage::Layer(fgid, tgid, l_msg) => {
                if tgid == DOMAIN_ID {
                    if let Ok(results) = layer.write().await.handle(fgid, l_msg).await {
                        handle(results, 0, &sender).await;
                    }
                }
            }
            ReceiveMessage::Rpc(uid, params, _is_ws) => {
                if let Ok(results) = rpc_handler.handle(params).await {
                    handle(results, uid, &sender).await;
                }
            }
            ReceiveMessage::NetworkLost => {
                //
            }
        }
    }

    Ok(())
}

#[inline]
async fn handle(handle_result: HandleResult, uid: u64, sender: &Sender<SendMessage>) {
    let HandleResult {
        mut owns,
        mut rpcs,
        mut groups,
        mut layers,
        mut networks,
    } = handle_result;

    loop {
        if rpcs.len() != 0 {
            let msg = rpcs.remove(0);
            sender
                .send(SendMessage::Rpc(uid, msg, false))
                .await
                .expect("TDN channel closed");
        } else {
            break;
        }
    }

    loop {
        if networks.len() != 0 {
            let msg = networks.remove(0);
            sender
                .send(SendMessage::Network(msg))
                .await
                .expect("TDN channel closed");
        } else {
            break;
        }
    }

    loop {
        if owns.len() != 0 {
            let msg = owns.remove(0);
            sender
                .send(SendMessage::Own(msg))
                .await
                .expect("TDN channel closed");
        } else {
            break;
        }
    }

    loop {
        if groups.len() != 0 {
            let msg = groups.remove(0);
            sender
                .send(SendMessage::Group(msg))
                .await
                .expect("TDN channel closed");
        } else {
            break;
        }
    }

    loop {
        if layers.len() != 0 {
            let (tgid, msg) = layers.remove(0);
            sender
                .send(SendMessage::Layer(tgid, msg))
                .await
                .expect("TDN channel closed");
        } else {
            break;
        }
    }
}

#[inline]
pub fn init_log(mut db_path: PathBuf) {
    db_path.push(DEFAULT_LOG_FILE);

    #[cfg(debug_assertions)]
    CombinedLogger::init(vec![simplelog::TermLogger::new(
        LevelFilter::Debug,
        LogConfig::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )])
    .unwrap();

    #[cfg(not(debug_assertions))]
    CombinedLogger::init(vec![simplelog::WriteLogger::new(
        LevelFilter::Debug,
        LogConfig::default(),
        std::fs::File::create(db_path).unwrap(),
    )])
    .unwrap();
}
