use serde::{Deserialize, Serialize};

use crate::executor::types::TxBody;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    ReallocateMemory(ReallocateMemory),
    GetValueAt(GetValueAt),
    SubmitTx(SubmitTx),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReallocateMemory {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetValueAt {
    pub addr: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitTx {
    pub tx_body: TxBody,
}
