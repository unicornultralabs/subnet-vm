use crate::block_stm::get_val;
use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use crate::examples::alloc::{self};
use crate::executor::process_tx;
use crate::executor::types::TxResult;
use crate::svm::{primitive_types::SVMPrimitives, svm::SVM};
use events::{GetValueAt, Message, SubmitTx};
use futures::lock::Mutex;
use futures::{SinkExt, StreamExt};
use log::{error, info};
use serde_json::json;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, WebSocketStream};

pub mod events;

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
                    Message::SubmitTx(SubmitTx { tx_body }) => {
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
                    Message::GetValueAt(GetValueAt { addr }) => {
                        tokio::spawn(async move {
                            let mut send = send_clone.lock().await;
                            let result = get_val(tm_loop, addr.clone());
                            // transform to confirmed transaction
                            let query_result = json!({
                                "addr": addr,
                                "value": result
                            });
                            if let Err(e) = send.send(query_result.to_string().into()).await {
                                println!("failed to send query balance result: {}", e);
                            }
                        });
                    }
                    Message::ReallocateMemory(_) => {
                        tokio::spawn(async move {
                            alloc::alloc_incremental(tm_loop.clone(), 0, 1_000_000).await;
                            alloc::alloc_duangua(tm_loop.clone(), 1_000_001, 1_000_002).await;
                            info!("reallocated memory");
                        });
                    }
                }
            }
        });
    }

    info!("ws disconnected");
}

#[cfg(test)]
mod tests {
    use events::ReallocateMemory;

    use crate::executor::types::TxBody;

    use super::*;

    #[test]
    fn sample_events_json() {
        let events = vec![
            // events send by frontend
            Message::GetValueAt(GetValueAt {
                addr: "0x1".to_string(),
            }),
            Message::ReallocateMemory(ReallocateMemory {}),
            Message::SubmitTx(SubmitTx {
                tx_body: TxBody {
                    tx_hash: "0xtxhash".to_string(),
                    code_hash: "0xcodehash".to_string(),
                    objs: vec![],
                    args: vec![],
                },
            }),
        ];

        let events_json = events.iter().map(|e| serde_json::to_string(&e).unwrap());
        for ejson in events_json {
            println!("{:?}", ejson)
        }
    }
}
