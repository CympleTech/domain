use std::path::PathBuf;
use tdn::types::{
    group::GroupId,
    message::{RecvType, SendType},
    primitives::{HandleResult, PeerId, Result},
};

use domain_types::{LayerPeerEvent, LayerServerEvent, DOMAIN_ID};

use crate::models::User;
use crate::{DEFAULT_PROVIDER_NAME, DEFAULT_PROVIDER_PROXY};

/// Domain server to peer.
#[inline]
pub(crate) fn add_server_layer(
    results: &mut HandleResult,
    addr: PeerId,
    event: LayerServerEvent,
    tgid: GroupId,
) -> Result<()> {
    let data = bincode::serialize(&event)?;
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
                let event: LayerPeerEvent = bincode::deserialize(&bytes)?;

                // TODO check proof. date if from fgid.

                match event {
                    LayerPeerEvent::Check => {
                        let status = LayerServerEvent::Status(
                            DEFAULT_PROVIDER_NAME.to_owned(),
                            DEFAULT_PROVIDER_PROXY,
                        );

                        add_server_layer(&mut results, addr, status, fgid)?;
                        println!("------ DEBUG DOMAIN SERVICE IS OK");
                    }
                    LayerPeerEvent::Search(name) => {
                        if let Ok(user) = User::search(&self.base, &name).await {
                            add_server_layer(&mut results, addr, user.to_info(), fgid)?;
                        } else {
                            add_server_layer(
                                &mut results,
                                addr,
                                LayerServerEvent::None(name),
                                fgid,
                            )?;
                        }
                    }
                    LayerPeerEvent::Register(name, bio, avatar) => {
                        let mut user = User::new(name.clone(), addr, bio, avatar);
                        let is_ok = user.insert(&self.base).await.is_ok();
                        add_server_layer(
                            &mut results,
                            addr,
                            LayerServerEvent::Result(name, is_ok),
                            fgid,
                        )?;
                    }
                    LayerPeerEvent::Update(name, bio, avatar) => {
                        let user = User::get_by_name(&self.base, &name).await?;
                        if user.pid == addr {
                            User::update(&user.id, &bio, &avatar, &self.base).await?;
                        }
                    }
                    LayerPeerEvent::Suspend(name) => {
                        let user = User::get_by_name(&self.base, &name).await?;
                        if user.pid == addr {
                            User::active(&user.id, false).await?;
                            add_server_layer(
                                &mut results,
                                addr,
                                LayerServerEvent::Actived(name, false),
                                fgid,
                            )?;
                        }
                    }
                    LayerPeerEvent::Active(name) => {
                        let user = User::get_by_name(&self.base, &name).await?;
                        if user.pid == addr {
                            User::active(&user.id, true).await?;
                            add_server_layer(
                                &mut results,
                                addr,
                                LayerServerEvent::Actived(name, true),
                                fgid,
                            )?;
                        }
                    }
                    LayerPeerEvent::Delete(name) => {
                        if let Ok(user) = User::get_by_name(&self.base, &name).await {
                            if user.pid == addr {
                                User::delete(&user.id, &self.base).await?;
                            }
                        }
                        add_server_layer(
                            &mut results,
                            addr,
                            LayerServerEvent::Deleted(name),
                            fgid,
                        )?;
                    }
                    LayerPeerEvent::Request(_name, _rname, _remark) => {}
                }
            }
            RecvType::Delivery(_t, _tid, _is_ok) => {
                // MAYBE
            }
        }

        Ok(results)
    }
}
