use crate::block_stm::svm_memory::{retry_transaction, retry_transaction_with_timers, SVMMemory};
use crate::svm::{builtins::TRANSFER_CODE_ID, primitive_types::SVMPrimitives, svm::SVM};
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::{task::JoinSet, time::Instant};

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
                    Ok((term, _stats, _diags)) => {
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
                                return Ok(result);
                            }
                            _ => return Err("unexpected type of result".to_owned()),
                        };
                    }
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

pub async fn reverse_transfer(tm: Arc<SVMMemory>, svm: Arc<SVM>, a: u32, b: u32) {
    let now = Instant::now();
    let mut set = JoinSet::new();
    let txs_timers: Arc<RwLock<HashMap<u64, (u128, u128, u128)>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let mut total_txs = 0;

    for i in (a + 1..=b).rev() {
        let txs_timers = txs_timers.clone();
        let svm = svm.clone();
        let tm = tm.clone();
        let from_key = format!("0x{}", i);
        let from_key_vec = from_key.as_bytes().to_vec();
        for j in a..i {
            let txs_timers = txs_timers.clone();
            let txid: u64 = ((i as u64) << 32) + (j as u64);

            total_txs += 1;

            let tm = tm.clone();
            let svm = svm.clone();
            let from_key = from_key.clone();
            let from_key_vec = from_key_vec.clone();
            let to_key = format!("0x{}", j);
            let to_key_vec = to_key.as_bytes().to_vec();

            set.spawn(async move {
                let (result, timers) = retry_transaction_with_timers(tm, |txn| {
                    let (mut vm_mrs, mut mem_mrs) = (0, 0);

                    let now = Instant::now();
                    let from_value = match txn.read(from_key_vec.clone()) {
                        Some(value) => {
                            mem_mrs += now.elapsed().as_micros();
                            value
                        }
                        None => {
                            mem_mrs += now.elapsed().as_micros();
                            return (
                                Err(format!("key={} does not exist", from_key)),
                                (vm_mrs, mem_mrs),
                            );
                        }
                    };
                    let now = Instant::now();
                    let to_value = match txn.read(to_key_vec.clone()) {
                        Some(value) => {
                            mem_mrs += now.elapsed().as_micros();
                            value
                        }
                        None => {
                            mem_mrs += now.elapsed().as_micros();
                            return (
                                Err(format!("key={} does not exist", to_key)),
                                (vm_mrs, mem_mrs),
                            );
                        }
                    };
                    let amt = SVMPrimitives::U24(1).to_term();

                    let args = { Some(vec![from_value.to_term(), to_value.to_term(), amt]) };

                    let now = Instant::now();
                    let svm_result = svm.clone().run_code(TRANSFER_CODE_ID, args);
                    vm_mrs += now.elapsed().as_micros();

                    match svm_result {
                        Ok((term, _stats, _diags)) => {
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
                                    return (Ok(Some(result)), (vm_mrs, mem_mrs));
                                }
                                _ => {
                                    return (
                                        Err("unexpected type of result".to_owned()),
                                        (vm_mrs, mem_mrs),
                                    )
                                }
                            };
                        }
                        Err(e) => {
                            return (
                                Err(format!("svm execution failed err={}", e)),
                                (vm_mrs, mem_mrs),
                            )
                        }
                    };
                });
                if let Err(e) = result {
                    error!("from_key={} err={}", from_key.clone(), e);
                }
                tokio::spawn(async move {
                    txs_timers.write().await.insert(txid, timers);
                });
            });
        }
    }
    while let Some(_) = set.join_next().await {}
    info!(
        "finish transfering total_txs={} elapesed_microsec={}",
        total_txs,
        now.elapsed().as_micros()
    );
    {
        let mut stats_content = String::from("");
        stats_content.push_str(&format!("i,j,vm_mrs,mem_mrs,backoff_mrs\n"));
        let stats = txs_timers.read().await;
        for (txid, timers) in stats.iter() {
            let i = txid >> 32;
            let j = (txid << 32) >> 32;
            stats_content.push_str(&format!(
                "{},{},{},{},{}\n",
                i, j, timers.0, timers.1, timers.2
            ));
        }
        _ = fs::write("stat.csv", &stats_content);
    }
}
