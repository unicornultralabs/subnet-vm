use crate::examples::make_move::make_move;
use crate::{block_stm::svm_memory::SVMMemory, svm::svm::SVM};
use log::info;
use transfer::reverse_transfer;
use std::sync::Arc;

pub mod alloc;
pub mod make_move;
pub mod query;
pub mod transfer;

pub async fn run_example(tm: Arc<SVMMemory>, svm: Arc<SVM>) {
    let a = 0;
    let b = 100;

    alloc::alloc(tm.clone(), a, b).await;

    // info!("{:?}", make_move(tm, svm, 0).await);
    // query(tm.clone(), a, b);

    // transfer(tm.clone(), svm.clone(), a, b).await;
    reverse_transfer(tm.clone(), svm.clone(), a, b).await;

    // query(tm.clone(), a, b);
}
