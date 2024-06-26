use crate::examples::run_example;
use block_stm::svm_memory::SVMMemory;
use examples::alloc;
use std::sync::Arc;
use svm::svm::SVM;

pub mod block_stm;
pub mod examples;
pub mod executor;
pub mod svm;
pub mod ws;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let tm = Arc::new(SVMMemory::new());
    let svm = Arc::new(SVM::new());
    let addr = "0.0.0.0:9001";

    // run_example(tm.clone(), svm.clone(), 0, 100).await;

    ws::run_ws(&addr, tm, svm).await;
}
