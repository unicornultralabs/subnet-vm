use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use log::{error, info};
use std::sync::Arc;
use tokio::time::Instant;

pub fn query(tm: Arc<SVMMemory>, a: u32, b: u32) {
    let now = Instant::now();

    for i in a..=b {
        let tm = tm.clone();
        let key = format!("0x{}", i);
        let key_vec = key.clone().as_bytes().to_vec();
        if let Err(e) = retry_transaction(tm, |txn| {
            let from_value = match txn.read(key_vec.clone()) {
                Some(value) => value,
                None => return Err(format!("key={} does not exist", key)),
            };
            info!("key={} Result:{:?}", key.clone(), from_value);
            Ok(None)
        }) {
            error!("key={} err={}", key.clone(), e);
        }
    }
    info!(
        "finish query elapsed_microsec={}",
        now.elapsed().as_micros()
    );
}
