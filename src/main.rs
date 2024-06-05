use block_stm::svm_memory::{retry_transaction, SVMMemory};
use futures::lock::Mutex;
use futures::{SinkExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::accept_async;
use std::sync::Arc;
use svm::{builtins::TRANSFER_CODE_ID, primitive_types::SVMPrimitives, svm::SVM};
use tokio::net::{TcpListener, TcpStream};
use tokio::{task::JoinSet, time::Instant};


pub mod block_stm;
pub mod executor;
pub mod svm;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SubmitTransaction {
    hash: String,
    from: String,
    to: String,
    amount: u32,
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
    let addr = "127.0.0.1:9001";

    let a = 1;
    let b = 10;

    alloc(tm.clone(), a, b).await;
    run_ws(&addr, tm, svm).await;
}

async fn run_example(tm: Arc<SVMMemory>, svm: Arc<SVM>) {
    let a = 1;
    let b = 10;

    alloc(tm.clone(), a, b).await;
    query(tm.clone(), a, b);

    transfer(tm.clone(), svm.clone(), a, b);
    query(tm.clone(), a, b);
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

fn transfer(tm: Arc<SVMMemory>, svm: Arc<SVM>, a: u32, b: u32) {
    let now = Instant::now();

    for i in (a + 1..=b).rev() {
        let tm = tm.clone();
        let svm = svm.clone();
        // set.spawn(async move {
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
        // });
    }
    info!(
        "finish transfer elapesed_microsec={}",
        now.elapsed().as_micros()
    );
}

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

async fn run_ws(addr: &str, tm: Arc<SVMMemory>, svm: Arc<SVM>){
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    info!("web socket is running on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tm = tm.clone();
        let svm = svm.clone();
        tokio::spawn(handle_connection(stream, tm, svm));
    }
}

async fn handle_connection(raw_stream: TcpStream, tm: Arc<SVMMemory>, svm: Arc<SVM>) {
    let ws_stream = accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    let (write, mut read) = ws_stream.split();
    let ws_send = Arc::new(Mutex::new(write));

    while let Some(message) = read.next().await {
        match message {
            Ok(msg) => {
                if msg.is_text() || msg.is_binary() {
                    let text = msg.clone().into_text().unwrap();
                    let parsed: Vec<Message> = serde_json::from_str(&text).unwrap();
                    for item in parsed {
                        let tm_loop = Arc::clone(&tm);
                        let svm_loop = Arc::clone(&svm);
                        let send_clone = Arc::clone(&ws_send);
                        match item {
                            Message::SubmitTransaction(transaction) => {
                                tokio::spawn(async move {
                                    let result = process_transaction(transaction, tm_loop, svm_loop);
                                    let mut send = send_clone.lock().await;
                                    match result {
                                        Ok(svm) => {
                                            if let Ok(json_result) = serde_json::to_string(&svm) {
                                                if let Err(e) = send.send(json_result.into()).await {
                                                    println!("failed to send svm result: {}", e);
                                                }
                                            } else {
                                                if let Err(e) = send.send("failed to convert svm result to json string".into()).await {
                                                    println!("failed to convert svm result to json string: {}", e);
                                                }
                                            }
                                        },
                                        Err(err) => {
                                            if let Err(e) = send.send(err.clone().into()).await {
                                                println!("svm err: {}, ws send message err: {}", err, e);
                                            }
                                        },
                                    }
                                });
                            }
                            // _ => {
                            //     error!("Unknown message type");
                            // }
                        }
                    }
                    // write.send(msg.clone()).await.unwrap();
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
    svm: Arc<SVM>
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
        let amt = SVMPrimitives::U24(1).to_term();

        let args = { Some(vec![from_value.to_term(), to_value.to_term(), amt]) };
        match svm.clone().run_code(TRANSFER_CODE_ID, args) {
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
