use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use crate::svm::primitive_types::SVMPrimitives;
use log::{error, info};
use std::sync::Arc;
use tokio::{task::JoinSet, time::Instant};

pub async fn alloc_incremental(tm: Arc<SVMMemory>, a: u32, b: u32) {
    let now = Instant::now();
    let mut set = JoinSet::new();
    for i in a..=b {
        let tm = tm.clone();
        set.spawn(async move {
            let key = format!("0x{}", i);
            let key_vec = key.clone().as_bytes().to_vec();
            if let Err(e) = retry_transaction(tm, |txn| {
                let alloc_amt = SVMPrimitives::U24(i);
                txn.write(key_vec.clone(), alloc_amt.clone());
                Ok(alloc_amt)
            }) {
                error!("key={} err={}", key.clone(), e);
            }
        });
    }
    while let Some(_) = set.join_next().await {}
    info!(
        "finish allocation elapesed_microsec={}",
        now.elapsed().as_micros()
    );
}
