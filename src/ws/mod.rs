use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use crate::examples::alloc::alloc_incremental;
use crate::examples::make_move::make_move;
use crate::executor::process_tx;
use crate::executor::types::{TxBody, TxResult};
use crate::svm::{primitive_types::SVMPrimitives, svm::SVM};
use futures::lock::Mutex;
use futures::{SinkExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, WebSocketStream};

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
struct QueryBalance {
    code_hash: String,
    address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MakeMove {
    code_hash: String,
    address: String,
    step: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Message {
    SubmitTransaction(SubmitTransaction),
    QueryBalance(QueryBalance),
    MakeMove(MakeMove),
    SubmitTx(TxBody),
}

pub async fn run_ws(addr: &str, tm: Arc<SVMMemory>, svm: Arc<SVM>) {
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
                    // info!("Received message: {}", text);
                    let send_clone = Arc::clone(&ws_send);
                    if let Ok(message) = serde_json::from_str::<Message>(&text) {
                        let tm_loop = Arc::clone(&tm);
                        let svm_loop = Arc::clone(&svm);
                        match message {
                            Message::SubmitTx(tx_body) => {
                                tokio::spawn(async move {
                                    let mut send = send_clone.lock().await;
                                    let tx_result =
                                        match process_tx(tx_body.clone(), tm_loop, svm_loop) {
                                            Ok(ret_val) => TxResult {
                                                code_hash: tx_body.code_hash,
                                                tx_hash: tx_body.tx_hash,
                                                ret_value: Some(ret_val),
                                                status: true,
                                                errs: None,
                                            },
                                            Err(e) => TxResult {
                                                tx_hash: tx_body.tx_hash,
                                                code_hash: tx_body.code_hash,
                                                status: false,
                                                ret_value: None,
                                                errs: Some(e.clone().into()),
                                            },
                                        };
                                    let json_tx_result = serde_json::to_string(&tx_result).unwrap();
                                    if let Err(e) = send.send(json_tx_result.into()).await {
                                        println!("failed to send confirmed transaction: {}", e);
                                    }
                                });
                            }
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
                            }
                            Message::QueryBalance(query) => {
                                tokio::spawn(async move {
                                    let mut send = send_clone.lock().await;
                                    let result = process_query_balance(query.clone(), tm_loop);
                                    match result {
                                        Ok(ret_val) => {
                                            // transform to confirmed transaction
                                            let confirmed_transaction = ConfirmedTransaction {
                                                code_hash: query.code_hash,
                                                tx_hash: "".into(),
                                                ret_value: Some(ret_val),
                                                status: true,
                                                errs: None,
                                            };
                                            if let Ok(json_result) =
                                                serde_json::to_string(&confirmed_transaction)
                                            {
                                                if let Err(e) = send.send(json_result.into()).await
                                                {
                                                    println!(
                                                        "failed to send query balance result: {}",
                                                        e
                                                    );
                                                }
                                            } else {
                                                if let Err(e) = send.send("failed to convert query balance result to json string".into()).await {
                                                    println!("failed to convert query balance result to json string: {}", e);
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            if let Err(e) = send.send(err.into()).await {
                                                println!(
                                                    "failed to send query balance error: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                });
                            }
                            Message::MakeMove(m) => {
                                tokio::spawn(async move {
                                    let mut a_or_b = 0;
                                    match m.address == format!("0x0") {
                                        true => a_or_b = 0,
                                        false => a_or_b = 1,
                                    }
                                    let mut send = send_clone.lock().await;
                                    let result = make_move(tm_loop, svm_loop, a_or_b).await;
                                    match result {
                                        Ok(ret_val) => {
                                            // transform to confirmed transaction
                                            let confirmed_transaction = ConfirmedTransaction {
                                                code_hash: "0xduangua".into(),
                                                tx_hash: "".into(),
                                                ret_value: Some(ret_val),
                                                status: true,
                                                errs: None,
                                            };
                                            if let Ok(json_result) =
                                                serde_json::to_string(&confirmed_transaction)
                                            {
                                                if let Err(e) = send.send(json_result.into()).await
                                                {
                                                    println!(
                                                        "failed to send query balance result: {}",
                                                        e
                                                    );
                                                }
                                            } else {
                                                if let Err(e) = send.send("failed to convert query balance result to json string".into()).await {
                                                    println!("failed to convert query balance result to json string: {}", e);
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            if let Err(e) = send.send(err.into()).await {
                                                println!(
                                                    "failed to send query balance error: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                });
                            }
                            _ => {
                                let mut send = send_clone.lock().await;
                                if let Err(e) = send.send("unsupported message".into()).await {
                                    println!("failed to send unsupported message: {}", e);
                                }
                            }
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
    // Handle WebSocket disconnection
    on_ws_disconnected(tm, 1, 1_000_000).await;
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
        match svm.clone().run_code(&transaction.code_hash, args) {
            Ok((term, _stats, _diags)) => {
                let result = SVMPrimitives::from_term(term.clone());
                match result {
                    SVMPrimitives::Tup(ref els) => {
                        let (from_val, to_val) = (els[0].clone(), els[1].clone());
                        txn.write(from_key_vec.clone(), from_val);
                        txn.write(to_key_vec.clone(), to_val);
                        return Ok(result);
                    }
                    _ => return Err("unexpected type of result".to_string()),
                };
            }
            Err(e) => Err(format!("svm execution failed err={}", e)),
        }
    });

    match result {
        Ok(res) => Ok(res),
        Err(e) => Err(format!("from_key={} err={}", from_key, e)),
    }
}

fn process_query_balance(query: QueryBalance, tm: Arc<SVMMemory>) -> Result<SVMPrimitives, String> {
    let key_vec = query.address.clone().as_bytes().to_vec();
    retry_transaction(tm.clone(), |txn| {
        let return_value = match txn.read(key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", query.address)),
        };
        // info!("key={} Result:{:?}", query.address, return_value);
        Ok(return_value)
    })
}

async fn on_ws_disconnected(tm: Arc<SVMMemory>, a: u32, b: u32) {
    println!("WebSocket connection closed or disconnected, reverting memory changes...");
    alloc_incremental(tm.clone(), a, b).await;
}
