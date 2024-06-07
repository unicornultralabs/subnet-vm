use crate::svm::primitive_types::SVMPrimitives;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TxBody {
    pub tx_hash: String,
    /// the code hash
    pub code_hash: String,
    /// the object we want to run the code with
    pub objs: Vec<String>,
    /// arguments for code execution
    pub args: Vec<SVMPrimitives>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TxResult {
    pub tx_hash: String,
    pub code_hash: String,
    pub status: bool,
    pub ret_value: Option<SVMPrimitives>,
    pub errs: Option<String>,
}
