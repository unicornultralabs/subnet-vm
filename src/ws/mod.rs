use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use crate::examples::alloc::{self};
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
    ReallocateMemory(()),
    QueryBalance(QueryBalance),
    SubmitTx(TxBody),
}

pub async fn run_ws(addr: &str, tm: Arc<SVMMemory>, svm: Arc<SVM>) {
    alloc::alloc_incremental(tm.clone(), 0, 1_000_000).await;
    alloc::alloc_duangua(tm.clone(), 1_000_001, 1_000_002).await;

    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    info!("web socket is running on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tm = tm.clone();
        let svm = svm.clone();

        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(stream) => {
                    info!("connecct");
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

    while let Some(Ok(msg)) = read.next().await {
        let send_clone = Arc::clone(&ws_send);
        let tm_loop = Arc::clone(&tm);
        let svm_loop = Arc::clone(&svm);
        tokio::spawn(async move {
            if msg.is_text() || msg.is_binary() {
                let text = msg.clone().into_text().unwrap();

                let message = match serde_json::from_str::<Message>(&text) {
                    Ok(message) => message,
                    Err(_) => {
                        let mut send = send_clone.lock().await;
                        _ = send
                            .send(format!("VM does not support message: {}", text).into())
                            .await;
                        return ();
                    }
                };

                info!("Received message: {:?}", message);
                match message {
                    Message::SubmitTx(tx_body) => {
                        let mut send = send_clone.lock().await;
                        let tx_result = match process_tx(tx_body.clone(), tm_loop, svm_loop) {
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
                        _ = send.send(json_tx_result.into()).await;
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
                                        if let Err(e) = send.send(json_result.into()).await {
                                            println!("failed to send query balance result: {}", e);
                                        }
                                    } else {
                                        if let Err(e) = send.send("failed to convert query balance result to json string".into()).await {
                                            println!("failed to convert query balance result to json string: {}", e);
                                        }
                                    }
                                }
                                Err(err) => {
                                    if let Err(e) = send.send(err.into()).await {
                                        println!("failed to send query balance error: {}", e);
                                    }
                                }
                            }
                        });
                    }
                    Message::ReallocateMemory(_) => {
                        tokio::spawn(async move {
                            alloc::alloc_incremental(tm_loop.clone(), 0, 1_000_000).await;
                            alloc::alloc_duangua(tm_loop.clone(), 1_000_001, 1_000_002).await;
                        });
                    }
                }
            }
        });
    }

    info!("ws disconnected");
}

fn process_query_balance(query: QueryBalance, tm: Arc<SVMMemory>) -> Result<SVMPrimitives, String> {
    let key_vec = query.address.clone().as_bytes().to_vec();
    retry_transaction(tm.clone(), |txn| {
        let return_value = match txn.read(key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", query.address)),
        };
        Ok(return_value)
    })
}
