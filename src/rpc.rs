use std::sync::Arc;
use tdn::types::{
    primitives::HandleResult,
    rpc::{json, RpcHandler},
};
use tokio::sync::RwLock;

use crate::layer::Layer;
use crate::models::User;

pub(crate) struct RpcState {
    pub layer: Arc<RwLock<Layer>>,
}

pub(crate) fn new_rpc_handler(layer: Arc<RwLock<Layer>>) -> RpcHandler<RpcState> {
    let mut handler = RpcHandler::new(RpcState { layer });

    handler.add_method("echo", |_, state: Arc<RpcState>| async move {
        let layer = state.layer.read().await;

        Ok(HandleResult::rpc(json!({
            "name": layer.name,
            "peer_id": layer.pid.to_hex(),
            "proxy": layer.proxy,
        })))
    });

    handler.add_method("list-users", |_, state: Arc<RpcState>| async move {
        let users = User::list(&state.layer.read().await.base).await?;
        let mut vecs = vec![];
        for user in users {
            vecs.push(user.to_rpc());
        }
        Ok(HandleResult::rpc(json!(vecs)))
    });

    handler
}
