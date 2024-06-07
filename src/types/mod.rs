use serde::{Deserialize, Serialize};

use crate::svm::primitive_types::SVMPrimitives;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitTransaction {
    pub tx_hash: String,
    pub code_hash: String,
    pub from: String,
    pub to: String,
    pub amount: u32,
}

#[derive(Serialize)]
pub struct ConfirmedTransaction {
    pub tx_hash: String,
    pub code_hash: String,
    pub status: bool,
    pub ret_value: Option<SVMPrimitives>,
    pub errs: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryBalance {
    pub code_hash: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MakeMove {
    pub code_hash: String,
    pub address: String,
    pub step: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum WsMessage {
    SubmitTransaction(SubmitTransaction),
    QueryBalance(QueryBalance),
    MakeMove(MakeMove),
}
