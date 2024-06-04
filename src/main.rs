use std::{sync::Arc, thread::spawn};

use block_stm::svm_memory::{retry_transaction, SVMMemory};
use log::info;
use svm::{builtins::ADD_CODE, primitive_types::SVMPrimitives};
use tokio::{task::JoinSet, time::Instant};

pub mod block_stm;
pub mod executor;
pub mod svm;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // if let Some((term, _stats, diags)) =
    //     svm::run_code(PARALLEL_HELLO_WORLD_CODE, None, None).expect("run code err")
    // {
    //     eprint!("{diags}");
    //     println!("Result:\n{}", term.display_pretty(0));
    // }

    // initially set value
    let tm = Arc::new(SVMMemory::new());
    let mut set = JoinSet::new();
    let now = Instant::now();
    info!("start allocation");
    for i in 1..=100000 {
        let tm = tm.clone();
        set.spawn(async move {
            let key = format!("0x{}", i).as_bytes().to_vec();
            retry_transaction(tm, |txn| {
                txn.write(key.clone(), SVMPrimitives::U24(i));
            });
        });
    }
    while let Some(_) = set.join_next().await {}
    info!("finish allocation elapesed_microsec={}", now.elapsed().as_micros());

    // // execute with vm and store back
    // retry_transaction(&tm, |txn| {
    //     if let Some(value) = txn.read(key.clone()) {
    //         let args = {
    //             let amt = SVMPrimitives::U24(100).to_term();
    //             Some(vec![value.to_term(), amt])
    //         };

    //         match svm::run_code(ADD_CODE, Some("add"), args).expect("run code err") {
    //             Some((term, _stats, diags)) => {
    //                 eprint!("{diags}");
    //                 println!("Result:\n{}", term.display_pretty(0));
    //                 txn.write(key.clone(), SVMPrimitives::from_term(term));
    //             }
    //             None => {
    //                 eprint!("svm execution failed");
    //             }
    //         }
    //     }
    // });
}
