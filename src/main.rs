use block_stm::svm_memory::{retry_transaction, SVMMemory};
use futures::{SinkExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use svm::{
    builtins::{ADD_CODE_ID, SUB_CODE_ID, TRANSFER_CODE_ID},
    primitive_types::SVMPrimitives,
    svm::SVM,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio::{task::JoinSet, time::Instant};

pub mod block_stm;
pub mod executor;
pub mod svm;

#[derive(Serialize, Deserialize, Debug)]
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
                            }
                            Message::Transaction(transaction) => {
                                println!("Transaction: {:?}", transaction);
                                 // Process the transaction
                                 let tm = tm.clone();
                                 let svm = svm.clone();
                                 tokio::spawn(async move {
                                     process_transaction(
                                        tm,
                                        svm,
                                        "0xa32d35f82f8743c16f52f3050d4ee25fa731d99b".to_string(),
                                        transaction
                                    ).await;
                                 });
                            }
                        }
                    }
                    write.send(msg).await.unwrap();
                }
            }
            Err(e) => {
                eprintln!("Error processing message: {}", e);
                break;
            }
        }
    }
}

async fn process_transaction(
    tm: Arc<SVMMemory>,
    svm: Arc<SVM>,
    id: String,
    transaction: Transaction
) {
    let key_vec = id.clone().as_bytes().to_vec();
    if let Err(e) = retry_transaction(tm, |txn| {
        txn.write(key_vec.clone(), SVMPrimitives::U24(transaction.amount));
        let value = match txn.read(key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", id)),
        };
        let amount = SVMPrimitives::U24(transaction.amount).to_term();
        let args = Some(vec![value.to_term(), amount]);

        // just the beginning, for further implementation: add and sub balance
        match svm.clone().run_code(ADD_CODE_ID, args) {
            Ok(Some((term, _stats, _diags))) => {
                txn.write(key_vec.clone(), SVMPrimitives::from_term(term.clone()));
                Ok(vec![SVMPrimitives::from_term(term)])
            }
            Ok(None) => Err(format!("svm execution failed err=none result")),
            Err(e) => Err(format!("svm execution failed err={}", e)),
        }
    }) {
        error!("transaction failed id={} err={}", id, e);
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
