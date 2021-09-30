use std::collections::HashMap;
use std::path::PathBuf;
use tdn::types::{
    group::GroupId,
    message::{RecvType, SendType},
    primitive::{HandleResult, PeerAddr, Result},
};
use tdn_did::Proof;

use domain_types::{LayerPeerEvent, LayerServerEvent, PeerEvent, ServerEvent, DOMAIN_ID};

use crate::models::{Name, Request};

/// Domain server to peer.
#[inline]
pub(crate) fn add_server_layer(
    results: &mut HandleResult,
    addr: PeerAddr,
    event: ServerEvent,
    tgid: GroupId,
) -> Result<()> {
    let proof = Proof::default();
    let data = bincode::serialize(&LayerServerEvent(event, proof))?;
    let s = SendType::Event(0, addr, data);
    results.layers.push((DOMAIN_ID, tgid, s));
    Ok(())
}

pub(crate) struct Layer {
    base: PathBuf,
}

impl Layer {
    pub(crate) async fn new(base: PathBuf) -> Result<Layer> {
        Ok(Layer { base })
    }

    pub(crate) async fn handle(&mut self, fgid: GroupId, msg: RecvType) -> Result<HandleResult> {
        let mut results = HandleResult::new();

        match msg {
            RecvType::Connect(..)
            | RecvType::Leave(..)
            | RecvType::Result(..)
            | RecvType::ResultConnect(..)
            | RecvType::Stream(..) => {
                info!("domain message nerver to here.")
            }
            RecvType::Event(addr, bytes) => {
                // server & client handle it.
                let LayerPeerEvent(event, proof) = bincode::deserialize(&bytes)?;

                match event {
                    PeerEvent::Check => {
                        add_server_layer(&mut results, addr, ServerEvent::Status, fgid)?;
                        println!("------ DEBUG DOMAIN SERVICE IS OK");
                    }
                    PeerEvent::Search(_name) => {}
                    PeerEvent::Register(_name, _bio, _avatar) => {}
                    PeerEvent::Update(_name, _bio, _avatar) => {}
                    PeerEvent::Request(_name, _rname, _remark) => {}
                    PeerEvent::Suspend(_name) => {}
                    PeerEvent::Active(_name) => {}
                    PeerEvent::Delete(_name) => {}
                }
            }
            RecvType::Delivery(_t, _tid, _is_ok) => {
                // MAYBE
            }
        }

        Ok(results)
    }
}
