use std::sync::Arc;
use tdn::types::{
    primitive::HandleResult,
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

    handler.add_method("echo", |_, params, _| async move {
        Ok(HandleResult::rpc(json!(params)))
    });

    handler.add_method("list-users", |_, _, state: Arc<RpcState>| async move {
        let users = User::list(&state.layer.read().await.base).await?;
        let mut vecs = vec![];
        for user in users {
            vecs.push(user.to_rpc());
        }
        Ok(HandleResult::rpc(json!(vecs)))
    });

    handler
}
