use std::path::PathBuf;
use tdn::types::{
    group::GroupId,
    message::{RecvType, SendType},
    primitive::{HandleResult, PeerAddr, Result},
};
use tdn_did::Proof;

use domain_types::{LayerPeerEvent, LayerServerEvent, PeerEvent, ServerEvent, DOMAIN_ID};

use crate::models::User;
use crate::{DEFAULT_PROVIDER_NAME, DEFAULT_PROVIDER_PROXY};

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
    pub base: PathBuf,
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
                let LayerPeerEvent(event, _proof) = bincode::deserialize(&bytes)?;

                // TODO check proof. date if from fgid.

                match event {
                    PeerEvent::Check => {
                        let status = ServerEvent::Status(
                            DEFAULT_PROVIDER_NAME.to_owned(),
                            DEFAULT_PROVIDER_PROXY,
                        );

                        add_server_layer(&mut results, addr, status, fgid)?;
                        println!("------ DEBUG DOMAIN SERVICE IS OK");
                    }
                    PeerEvent::Search(name) => {
                        if let Ok(user) = User::get_by_name(&self.base, &name).await {
                            add_server_layer(&mut results, addr, user.to_info(), fgid)?;
                        } else {
                            add_server_layer(&mut results, addr, ServerEvent::None(name), fgid)?;
                        }
                    }
                    PeerEvent::Register(name, bio, avatar) => {
                        let mut user = User::new(name.clone(), fgid, addr, bio, avatar);
                        let is_ok = user.insert(&self.base).await.is_ok();
                        add_server_layer(
                            &mut results,
                            addr,
                            ServerEvent::Result(name, is_ok),
                            fgid,
                        )?;
                    }
                    PeerEvent::Update(name, bio, avatar) => {
                        let user = User::get_by_name(&self.base, &name).await?;
                        if user.gid == fgid {
                            User::update(&user.id, &addr, &bio, &avatar, &self.base).await?;
                        }
                    }
                    PeerEvent::Suspend(name) => {
                        let user = User::get_by_name(&self.base, &name).await?;
                        if user.gid == fgid {
                            User::active(&user.id, false).await?;
                        }
                    }
                    PeerEvent::Active(name) => {
                        let user = User::get_by_name(&self.base, &name).await?;
                        if user.gid == fgid {
                            User::active(&user.id, true).await?;
                        }
                    }
                    PeerEvent::Delete(name) => {
                        let user = User::get_by_name(&self.base, &name).await?;
                        if user.gid == fgid {
                            User::delete(&user.id, &self.base).await?;
                        }
                    }
                    PeerEvent::Request(_name, _rname, _remark) => {}
                }
            }
            RecvType::Delivery(_t, _tid, _is_ok) => {
                // MAYBE
            }
        }

        Ok(results)
    }
}
