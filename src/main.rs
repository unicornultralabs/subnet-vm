use block_stm::svm_memory::{retry_transaction, SVMMemory};
use futures::{SinkExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use svm::{
    builtins::{ADD_CODE_ID, SUB_CODE_ID, TRANSFER_CODE_ID},
    primitive_types::SVMPrimitives,
    svm::SVM,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::accept_async;
use tokio::{task::JoinSet, time::Instant};

pub mod block_stm;
pub mod executor;
pub mod svm;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Transaction {
    from: String,
    to: String,
    amount: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Message {
    Address(String),
    Transaction(Transaction),
}

async fn handle_connection(raw_stream: TcpStream, tm: Arc<SVMMemory>, svm: Arc<SVM>) {
    let ws_stream = accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    let (mut write, mut read) = ws_stream.split();

    while let Some(message) = read.next().await {
        match message {
            Ok(msg) => {
                if msg.is_text() || msg.is_binary() {
                    let text = msg.clone().into_text().unwrap();
                    let parsed: Vec<Message> = serde_json::from_str(&text).unwrap();
                    for item in parsed {
                        match item {
                            Message::Address(address) => {
                                println!("Address: {}", address);
                                write.send(WsMessage::Text(address)).await.unwrap_or_else(|e| {
                                    eprintln!("Failed to send address: {}", e);
                                });
                            }
                            Message::Transaction(transaction) => {
                                println!("Transaction: {:?}", transaction);
                                // Process the transaction
                                let tm = tm.clone();
                                let svm = svm.clone();
                                tokio::spawn(async move {
                                    process_transaction(tm, svm, transaction).await;
                                });
                                // let response = json!(transaction).to_string();
                                // write.send(WsMessage::Text(transaction)).await.unwrap_or_else(|e| {
                                //     eprintln!("Failed to send transaction response: {}", e);
                                // });
                            }
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


async fn process_transaction(
    tm: Arc<SVMMemory>,
    svm: Arc<SVM>,
    transaction: Transaction
) {
    // let from_key = format!("0x{}", transaction.from);
    // let to_key = format!("0x{}", transaction.to);
    let from_key_vec = transaction.from.clone().as_bytes().to_vec();
    let to_key_vec = transaction.to.clone().as_bytes().to_vec();

    if let Err(e) = retry_transaction(tm.clone(), |txn| {
        let from_value = match txn.read(from_key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", transaction.from)),
        };
        let to_value = match txn.read(to_key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", transaction.to)),
        };
        let amt = SVMPrimitives::U24(transaction.amount).to_term();

        let args = Some(vec![from_value.to_term(), to_value.to_term(), amt]);
        match svm.clone().run_code(TRANSFER_CODE_ID, args) {
            Ok(Some((term, _stats, _diags))) => {
                println!(
                    "from_key={} Result:\n{}",
                    transaction.from.clone(),
                    term.display_pretty(0)
                );

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
        }
    }) {
        error!("from_key={} err={}", transaction.from.clone(), e);
    }
}



#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // initially set value
    let tm = Arc::new(SVMMemory::new());
    let svm = Arc::new(SVM::new());
    //let mut set = JoinSet::new();
    // let now = Instant::now();
    info!("start allocation");

    // Starting websocket server
    let addr = "127.0.0.1:9001";
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    info!("web socket is running on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tm = tm.clone();
        let svm = svm.clone();
        tokio::spawn(handle_connection(stream, tm, svm));
    }
}
