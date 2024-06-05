use block_stm::svm_memory::{retry_transaction, SVMMemory};
use log::{error, info};
use std::sync::Arc;
use svm::{
    builtins::{ADD_CODE_ID, SUB_CODE_ID},
    primitive_types::SVMPrimitives,
    svm::SVM,
};
use tokio::{task::JoinSet, time::Instant};

pub mod block_stm;
pub mod executor;
pub mod svm;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // initially set value
    let tm = Arc::new(SVMMemory::new());
    let svm = Arc::new(SVM::new());
    let mut set = JoinSet::new();
    let now = Instant::now();
    info!("start allocation");

    let a = 1;
    let b = 100000;

    for i in a..=b {
        let tm = tm.clone();
        let svm = svm.clone();
        set.spawn(async move {
            let key = format!("0x{}", i);
            let key_vec = key.clone().as_bytes().to_vec();
            if let Err(e) = retry_transaction(tm, |txn| {
                txn.write(key_vec.clone(), SVMPrimitives::U24(i));
                if let Some(value) = txn.read(key_vec.clone()) {
                    let amt = SVMPrimitives::U24(1).to_term();
                    let args = { Some(vec![value.to_term(), amt]) };
                    match svm.clone().run_code(ADD_CODE_ID, args) {
                        Ok(Some((term, _stats, _diags))) => {
                            // eprint!("i={} {diags}", i);
                            // println!("{:?} Result:\n{}", key.clone(), term.display_pretty(0));
                            txn.write(key_vec.clone(), SVMPrimitives::from_term(term));
                            return Ok(());
                        }
                        Ok(None) => return Err(format!("svm execution failed err=none result")),
                        Err(e) => return Err(format!("svm execution failed err={}", e)),
                    };
                }
                Ok(())
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

    // let mut trans_set = JoinSet::new();
    // let now = Instant::now();
    // info!("start transfering");
    // for i in (a + 1..=b).rev() {
    //     let svm = svm.clone();
    //     let tm = tm.clone();
    //     let from_key = format!("0x{}", i).as_bytes().to_vec();
    //     for j in a..i {
    //         info!("{} -> {}", i, j);
    //         let tm = tm.clone();
    //         let svm = svm.clone();
    //         let from_key = from_key.clone();
    //         let to_key = format!("0x{}", j).as_bytes().to_vec();

    //         trans_set.spawn(async move {
    //             retry_transaction(tm, |txn| {
    //                 let amt = SVMPrimitives::U24(1).to_term();

    //                 // sub
    //                 if let Some(value) = txn.read(from_key.clone()) {
    //                     let args = { Some(vec![value.to_term(), amt.clone()]) };

    //                     match svm
    //                         .clone()
    //                         .run_code(SUB_CODE_ID, args)
    //                         .expect("run code err")
    //                     {
    //                         Some((term, _stats, _diags)) => {
    //                             // eprint!("{diags}");
    //                             println!("Result:\n{}", term.display_pretty(0));
    //                             txn.write(from_key.clone(), SVMPrimitives::from_term(term));
    //                         }
    //                         None => {
    //                             eprint!("svm execution failed");
    //                         }
    //                     }
    //                 }

    //                 // add
    //                 if let Some(value) = txn.read(to_key.clone()) {
    //                     let args = { Some(vec![value.to_term(), amt]) };

    //                     match svm
    //                         .clone()
    //                         .run_code(ADD_CODE_ID, args)
    //                         .expect("run code err")
    //                     {
    //                         Some((term, _stats, _diags)) => {
    //                             // eprint!("{diags}");
    //                             println!("Result:\n{}", term.display_pretty(0));
    //                             txn.write(to_key.clone(), SVMPrimitives::from_term(term));
    //                         }
    //                         None => {
    //                             eprint!("svm execution failed");
    //                         }
    //                     }
    //                 }
    //             });
    //         });
    //     }
    // }
    // while let Some(_) = set.join_next().await {}
    // info!(
    //     "finish transfering elapesed_microsec={}",
    //     now.elapsed().as_micros()
    // );

    // for i in a..=b {
    //     let tm = tm.clone();
    //     let key = format!("0x{}", i).as_bytes().to_vec();
    //     retry_transaction(tm, |txn| {
    //         if let Some(val) = txn.read(key.clone()) {
    //             info!("{}: {:?}", i, val);
    //         }
    //     });
    // }
}
