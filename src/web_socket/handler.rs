use std::sync::Arc;
use log::info;

use crate::block_stm::svm_memory::SVMMemory;




pub async fn on_ws_disconnected(tm: Arc<SVMMemory>, a: u32, b: u32) {
    info!("Websocket disconnected, reverting memory changes...");
    // alloc(tm.clone(), a, b).await;
}