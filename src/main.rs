use block_stm::svm_memory::{retry_transaction, retry_transaction_with_timers, SVMMemory};
use futures::lock::Mutex;
use futures::{SinkExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use svm::{builtins::TRANSFER_CODE_ID, primitive_types::SVMPrimitives, svm::SVM};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::{task::JoinSet, time::Instant};
use tokio_tungstenite::{accept_async, WebSocketStream};
pub mod block_stm;
pub mod executor;
pub mod svm;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SubmitTransaction {
    tx_hash: String,
    code_hash: String,
    from: String,
    to: String,
    amount: u32,
}

#[derive(Serialize)]
struct ConfirmedTransaction {
    tx_hash: String,
    code_hash: String,
    status: bool,
    ret_value: Option<SVMPrimitives>,
    errs: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Message {
    SubmitTransaction(SubmitTransaction),
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let tm = Arc::new(SVMMemory::new());
    let svm = Arc::new(SVM::new());
    let addr = "0.0.0.0:9001";

    let a = 1;
    let b = 1_000_000;

    // allocate memory for testing purposes
    // alloc(tm.clone(), a, b).await;
    run_example(tm.clone(), svm.clone()).await;
    // run_ws(&addr, tm, svm).await;
}

async fn run_example(tm: Arc<SVMMemory>, svm: Arc<SVM>) {
    let a = 1;
    let b = 100;

    alloc(tm.clone(), a, b).await;
    // query(tm.clone(), a, b);

    transfer(tm.clone(), svm.clone(), a, b).await;
    // reverse_transfer(tm.clone(), svm.clone(), a, b).await;

    // query(tm.clone(), a, b);
}

async fn alloc(tm: Arc<SVMMemory>, a: u32, b: u32) {
    let now = Instant::now();
    let mut set = JoinSet::new();
    for i in a..=b {
        let tm = tm.clone();
        set.spawn(async move {
            let key = format!("0x{}", i);
            let key_vec = key.clone().as_bytes().to_vec();
            if let Err(e) = retry_transaction(tm, |txn| {
                txn.write(key_vec.clone(), SVMPrimitives::U24(i));
                Ok(None)
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

async fn transfer(tm: Arc<SVMMemory>, svm: Arc<SVM>, a: u32, b: u32) {
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
                match svm.clone().run_hvm_code(TRANSFER_CODE_ID, args) {
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

// async fn reverse_transfer(tm: Arc<SVMMemory>, svm: Arc<SVM>, a: u32, b: u32) {
//     let now = Instant::now();
//     // let mut set = JoinSet::new();
//     let txs_timers: Arc<RwLock<HashMap<u64, ((u128, u128, u128), u128, u128)>>> =
//         Arc::new(RwLock::new(HashMap::new()));

//     let mut total_txs = 0;

//     for i in (a + 1..=b).rev() {
//         let txs_timers = txs_timers.clone();
//         let svm = svm.clone();
//         let tm = tm.clone();
//         let from_key = format!("0x{}", i);
//         let from_key_vec = from_key.as_bytes().to_vec();
//         for j in a..i {
//             let txs_timers = txs_timers.clone();
//             let txid: u64 = ((i as u64) << 32) + (j as u64);

//             total_txs += 1;

//             let tm = tm.clone();
//             let svm = svm.clone();
//             let from_key = from_key.clone();
//             let from_key_vec = from_key_vec.clone();
//             let to_key = format!("0x{}", j);
//             let to_key_vec = to_key.as_bytes().to_vec();

//             // set.spawn(async move {
//             let (result, timers) = retry_transaction_with_timers(tm, |txn| {
//                 let mut mem_mrs = 0;

//                 let now = Instant::now();
//                 let from_value = match txn.read(from_key_vec.clone()) {
//                     Some(value) => {
//                         mem_mrs += now.elapsed().as_micros();
//                         value
//                     }
//                     None => {
//                         mem_mrs += now.elapsed().as_micros();
//                         return (
//                             Err(format!("key={} does not exist", from_key)),
//                             ((0, 0, 0), mem_mrs),
//                         );
//                     }
//                 };
//                 let now = Instant::now();
//                 let to_value = match txn.read(to_key_vec.clone()) {
//                     Some(value) => {
//                         mem_mrs += now.elapsed().as_micros();
//                         value
//                     }
//                     None => {
//                         mem_mrs += now.elapsed().as_micros();
//                         return (
//                             Err(format!("key={} does not exist", to_key)),
//                             ((0, 0, 0), mem_mrs),
//                         );
//                     }
//                 };
//                 let amt = SVMPrimitives::U24(1).to_term();

//                 let args = { Some(vec![from_value.to_term(), to_value.to_term(), amt]) };
//                 let (svm_result, vm_timers) =
//                     svm.clone().run_code_with_timers(TRANSFER_CODE_ID, args);

//                 match svm_result {
//                     Ok(Some((term, _stats, _diags))) => {
//                         // eprint!("i={} {diags}", i);
//                         // println!(
//                         //     "from_key={} Result:\n{}",
//                         //     from_key.clone(),
//                         //     term.display_pretty(0)
//                         // );

//                         let result = SVMPrimitives::from_term(term.clone());
//                         match result {
//                             SVMPrimitives::Tup(ref els) => {
//                                 let (from_val, to_val) = (els[0].clone(), els[1].clone());
//                                 txn.write(from_key_vec.clone(), from_val);
//                                 txn.write(to_key_vec.clone(), to_val);
//                                 return (Ok(Some(result)), (vm_timers, mem_mrs));
//                             }
//                             _ => {
//                                 return (
//                                     Err("unexpected type of result".to_owned()),
//                                     (vm_timers, mem_mrs),
//                                 )
//                             }
//                         };
//                     }
//                     Ok(None) => {
//                         return (
//                             Err(format!("svm execution failed err=none result")),
//                             (vm_timers, mem_mrs),
//                         )
//                     }
//                     Err(e) => {
//                         return (
//                             Err(format!("svm execution failed err={}", e)),
//                             (vm_timers, mem_mrs),
//                         )
//                     }
//                 };
//             });
//             if let Err(e) = result {
//                 error!("from_key={} err={}", from_key.clone(), e);
//             }
//             tokio::spawn(async move {
//                 txs_timers.write().await.insert(txid, timers);
//             });
//             // });
//         }
//     }
//     // while let Some(_) = set.join_next().await {}
//     info!(
//         "finish transfering total_txs={} elapesed_microsec={}",
//         total_txs,
//         now.elapsed().as_micros()
//     );
//     {
//         let mut stats_content = String::from("");
//         stats_content.push_str(&format!("i,j,vm_mrs,mem_mrs,backoff_mrs\n"));
//         let stats = txs_timers.read().await;
//         for (txid, timers) in stats.iter() {
//             let i = txid >> 32;
//             let j = (txid << 32) >> 32;
//             stats_content.push_str(&format!(
//                 "{},{},{:?},{},{}\n",
//                 i, j, timers.0, timers.1, timers.2
//             ));
//         }
//         _ = fs::write("stat.csv", &stats_content);
//     }
// }

fn query(tm: Arc<SVMMemory>, a: u32, b: u32) {
    let now = Instant::now();

    for i in a..=b {
        let tm = tm.clone();
        let key = format!("0x{}", i);
        let key_vec = key.clone().as_bytes().to_vec();
        if let Err(e) = retry_transaction(tm, |txn| {
            let from_value = match txn.read(key_vec.clone()) {
                Some(value) => value,
                None => return Err(format!("key={} does not exist", key)),
            };
            info!("key={} Result:{:?}", key.clone(), from_value);
            Ok(None)
        }) {
            error!("key={} err={}", key.clone(), e);
        }
    }
    info!(
        "finish query elapesed_microsec={}",
        now.elapsed().as_micros()
    );
}

async fn run_ws(addr: &str, tm: Arc<SVMMemory>, svm: Arc<SVM>) {
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    info!("web socket is running on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tm = tm.clone();
        let svm = svm.clone();

        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(stream) => {
                    tokio::spawn(handle_connection(stream, tm, svm));
                }
                Err(e) => {
                    error!("Error during the websocket handshake occurred: {}", e);
                }
            }
        });
        // tokio::spawn(handle_connection(stream, tm, svm));
    }
}

async fn handle_connection(
    ws_stream: WebSocketStream<TcpStream>,
    tm: Arc<SVMMemory>,
    svm: Arc<SVM>,
) {
    let (write, mut read) = ws_stream.split();
    let ws_send = Arc::new(Mutex::new(write));

    while let Some(message) = read.next().await {
        match message {
            Ok(msg) => {
                if msg.is_text() || msg.is_binary() {
                    let text = msg.clone().into_text().unwrap();
                    info!("Received message: {}", text);
                    // let parsed: Vec<Message> = serde_json::from_str(&text).unwrap();
                    let send_clone = Arc::clone(&ws_send);
                    if let Ok(message) = serde_json::from_str::<Message>(&text) {
                        let tm_loop = Arc::clone(&tm);
                        let svm_loop = Arc::clone(&svm);
                        // let send_clone = Arc::clone(&ws_send);
                        match message {
                            Message::SubmitTransaction(transaction) => {
                                tokio::spawn(async move {
                                    let mut send = send_clone.lock().await;
                                    let result =
                                        process_transaction(transaction.clone(), tm_loop, svm_loop);
                                    match result {
                                        Ok(ret_val) => {
                                            if let Ok(_json_result) =
                                                serde_json::to_string(&ret_val)
                                            {
                                                // transform to confirmed transaction
                                                let confirmed_transaction = ConfirmedTransaction {
                                                    code_hash: transaction.code_hash,
                                                    tx_hash: transaction.tx_hash,
                                                    ret_value: Some(ret_val),
                                                    status: true,
                                                    errs: None,
                                                };
                                                if let Ok(json_result) =
                                                    serde_json::to_string(&confirmed_transaction)
                                                {
                                                    if let Err(e) =
                                                        send.send(json_result.into()).await
                                                    {
                                                        println!("failed to send confirmed transaction: {}", e);
                                                    }
                                                } else {
                                                    if let Err(e) = send.send("failed to convert confirmed transaction to json string".into()).await {
                                                        println!("failed to convert confirmed transaction to json string: {}", e);
                                                    }
                                                }
                                            } else {
                                                if let Err(e) = send.send("failed to convert svm result to json string".into()).await {
                                                    println!("failed to convert svm result to json string: {}", e);
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            let confirmed_tx = ConfirmedTransaction {
                                                tx_hash: transaction.tx_hash,
                                                code_hash: transaction.code_hash,
                                                status: false,
                                                ret_value: None,
                                                errs: Some(err.clone().into()),
                                            };
                                            // send confirmed transaction
                                            if let Ok(json_result) =
                                                serde_json::to_string(&confirmed_tx)
                                            {
                                                if let Err(e) = send.send(json_result.into()).await
                                                {
                                                    println!(
                                                        "failed to send confirmed transaction: {}",
                                                        e
                                                    );
                                                }
                                            } else {
                                                if let Err(e) = send.send("failed to convert confirmed transaction to json string".into()).await {
                                                    println!("failed to convert confirmed transaction to json string: {}", e);
                                                }
                                            }
                                        }
                                    }
                                });
                            } // _ => {
                              //     error!("Unknown message type");
                              // }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error processing message: {}", e);
                break;
            }
        }
    }
}

fn process_transaction(
    transaction: SubmitTransaction,
    tm: Arc<SVMMemory>,
    svm: Arc<SVM>,
) -> Result<SVMPrimitives, std::string::String> {
    let tm = tm.clone();
    let svm = svm.clone();
    let from_key = transaction.from;
    let to_key = transaction.to;
    let from_key_vec = from_key.clone().as_bytes().to_vec();
    let to_key_vec = to_key.clone().as_bytes().to_vec();

    let result = retry_transaction(tm, |txn| {
        let from_value = match txn.read(from_key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", from_key)),
        };
        let to_value = match txn.read(to_key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", to_key)),
        };
        let amt = SVMPrimitives::U24(transaction.amount).to_term();

        let args = { Some(vec![from_value.to_term(), to_value.to_term(), amt]) };
        match svm.clone().run_hvm_code(&transaction.code_hash, args) {
            Ok(Some((term, _stats, _diags))) => {
                let result = SVMPrimitives::from_term(term.clone());
                match result {
                    SVMPrimitives::Tup(ref els) => {
                        let (from_val, to_val) = (els[0].clone(), els[1].clone());
                        txn.write(from_key_vec.clone(), from_val);
                        txn.write(to_key_vec.clone(), to_val);
                        return Ok(Some(result));
                    }
                    _ => return Err("unexpected type of result".to_string()),
                };
            }
            Ok(None) => Err("svm execution failed err=none result".to_string()),
            Err(e) => Err(format!("svm execution failed err={}", e)),
        }
    });

    match result {
        Ok(Some(res)) => Ok(res),
        Ok(None) => Err(format!("from_key={} did not produce a result", from_key)),
        Err(e) => Err(format!("from_key={} err={}", from_key, e)),
    }
}
