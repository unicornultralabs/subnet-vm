use crate::examples::query::query;
use crate::{block_stm::svm_memory::SVMMemory, svm::svm::SVM};
use make_move::make_move;
use std::sync::Arc;
use transfer::reverse_transfer;

pub mod alloc;
pub mod make_move;
pub mod query;
pub mod transfer;

pub async fn run_example(tm: Arc<SVMMemory>, svm: Arc<SVM>) {
    let a = 1;
    let b = 10;

    // alloc::alloc_incremental(tm.clone(), a, b).await;
    // alloc::alloc_duangua(tm.clone(), a, b).await;
    alloc::alloc_incremental(tm.clone(), 0, 1_000_000).await;
    alloc::alloc_duangua(tm.clone(), 1_000_001, 1_000_002).await;

    // info!("{:?}", make_move(tm, svm, 0).await);
    query(tm.clone(), a, b);

    // transfer(tm.clone(), svm.clone(), a, b).await;
    // reverse_transfer(tm.clone(), svm.clone(), a, b).await;
    _ = make_move(tm.clone(), svm.clone(), 1);
    query(tm.clone(), a, b);
}
