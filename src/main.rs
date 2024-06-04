use std::{sync::Arc, thread::spawn};

use block_stm::svm_memory::{retry_transaction, SVMMemory};
use log::info;
use svm::{
    builtins::{ADD_CODE, SUB_CODE},
    primitive_types::SVMPrimitives,
};
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

    let a = 1;
    let b = 100000;

    for i in a..=b {
        let tm = tm.clone();
        set.spawn(async move {
            let key = format!("0x{}", i).as_bytes().to_vec();
            retry_transaction(tm, |txn| {
                txn.write(key.clone(), SVMPrimitives::U24(i));
            });
        });
    }
    while let Some(_) = set.join_next().await {}
    info!(
        "finish allocation elapesed_microsec={}",
        now.elapsed().as_micros()
    );

    let mut trans_set = JoinSet::new();
    let now = Instant::now();
    info!("start transfering");
    for i in (a + 1..=b).rev() {
        let tm = tm.clone();
        let from_key = format!("0x{}", i).as_bytes().to_vec();
        for j in a..i {
            let tm = tm.clone();
            let from_key = from_key.clone();
            let to_key = format!("0x{}", j).as_bytes().to_vec();

            trans_set.spawn(async move {
                retry_transaction(tm, |txn| {
                    let amt = SVMPrimitives::U24(1).to_term();

                    // sub
                    if let Some(value) = txn.read(from_key.clone()) {
                        let args = { Some(vec![value.to_term(), amt.clone()]) };

                        match svm::run_code(SUB_CODE, args).expect("run code err") {
                            Some((term, _stats, diags)) => {
                                eprint!("{diags}");
                                println!("Result:\n{}", term.display_pretty(0));
                                txn.write(from_key.clone(), SVMPrimitives::from_term(term));
                            }
                            None => {
                                eprint!("svm execution failed");
                            }
                        }
                    }

                    // add
                    if let Some(value) = txn.read(to_key.clone()) {
                        let args = { Some(vec![value.to_term(), amt]) };

                        match svm::run_code(ADD_CODE, args).expect("run code err") {
                            Some((term, _stats, diags)) => {
                                eprint!("{diags}");
                                println!("Result:\n{}", term.display_pretty(0));
                                txn.write(to_key.clone(), SVMPrimitives::from_term(term));
                            }
                            None => {
                                eprint!("svm execution failed");
                            }
                        }
                    }
                });
            });
        }
    }
    while let Some(_) = set.join_next().await {}
    info!(
        "finish transfering elapesed_microsec={}",
        now.elapsed().as_micros()
    );
}
