use crate::block_stm::{get_val, svm_memory::SVMMemory};
use log::{error, info};
use std::sync::Arc;
use tokio::time::Instant;

pub fn query(tm: Arc<SVMMemory>, a: u32, b: u32) {
    let now = Instant::now();

    for i in a..=b {
        let tm = tm.clone();
        let key = format!("0x{}", i);
        match get_val(tm, key.clone()) {
            Some(val) => {
                info!("key={} Result:{:?}", key.clone(), val);
            }
            None => {
                error!("key={} Value not found", key.clone());
            }
        }
    }
    info!(
        "finish query elapsed_microsec={}",
        now.elapsed().as_micros()
    );
}
