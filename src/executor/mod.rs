use log::{error, info};
use tokio::{task::JoinSet, time::Instant};

use crate::{
    block_stm::svm_memory::{retry_transaction, SVMMemory},
    svm::{builtins::TRANSFER_CODE_ID, primitive_types::SVMPrimitives, svm::SVM},
};
use std::sync::Arc;

pub async fn alloc(tm: Arc<SVMMemory>, start: u32, b: u32) {
    let now = Instant::now();
    let mut set = JoinSet::new();
    for i in start..=b {
        let tm = tm.clone();
        set.spawn(async move {
            let key = format!("0x{}", i);
            let key_vec = key.clone().as_bytes().to_vec();
            if let Err(e) = retry_transaction(tm, |txn| {
                txn.write(key_vec.clone(), SVMPrimitives::U24(0));
                Ok(None)
            }) {
                error!("key={} err={}", key.clone(), e);
            }
        });
    }
    while let Some(_) = set.join_next().await {}
    info!(
        "finish allocation, {} micro seconds elapsed",
        now.elapsed().as_micros()
    );
}


pub async fn transfer(tm: Arc<SVMMemory>, svm: Arc<SVM>, a: u32, b: u32) {
    let now = Instant::now();

    let mut set = JoinSet::new();
    for i in (a + 1..=b).rev() {
        let tm = tm.clone();
        let svm = svm.clone();
        set.spawn(async move {
            let from_key = format!("0x{}", i);
            let to_key = format!("0x{}", i - 1);
            let from_key_vec = from_key.clone().as_bytes().to_vec();
            let to_key_vec = to_key.clone().as_bytes().to_vec();
            if let Err(e) = retry_transaction(tm, |txn| {
                let from_value = match txn.read(from_key_vec.clone()) {
                    Some(value) => value,
                    None => return Err(format!("key={} does not exist", from_key)),
                };
                let to_value = match txn.read(to_key_vec.clone()) {
                    Some(value) => value,
                    None => return Err(format!("key={} does not exist", to_key)),
                };
                let amt = SVMPrimitives::U24(1).to_term();

                let args = { Some(vec![from_value.to_term(), to_value.to_term(), amt]) };
                match svm.clone().run_code(TRANSFER_CODE_ID, args) {
                    Ok(Some((term, _stats, _diags))) => {
                        // eprint!("i={} {diags}", i);
                        // println!(
                        //     "from_key={} Result:\n{}",
                        //     from_key.clone(),
                        //     term.display_pretty(0)
                        // );

                        let result = SVMPrimitives::from_term(term.clone());
                        match result {
                            SVMPrimitives::Tup(ref els) => {
                                let (from_val, to_val) = (els[0].clone(), els[1].clone());
                                txn.write(from_key_vec.clone(), from_val);
                                txn.write(to_key_vec.clone(), to_val);
                                return Ok(Some(result));
                            }
                            _ => return Err("unexpected type of result".to_owned()),
                        };
                    }
                    Ok(None) => return Err(format!("svm execution failed err=none result")),
                    Err(e) => return Err(format!("svm execution failed err={}", e)),
                };
            }) {
                error!("from_key={} err={}", from_key.clone(), e);
            }
        });
    }
    while let Some(_) = set.join_next().await {}
    info!(
        "finish transfer elapesed_microsec={}",
        now.elapsed().as_micros()
    );
}