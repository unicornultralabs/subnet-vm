use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use crate::executor::process_tx;
use crate::executor::types::TxBody;
use crate::svm::builtins::DUANGUA_CODE_ID;
use crate::svm::{primitive_types::SVMPrimitives, svm::SVM};
use std::sync::Arc;

pub fn make_move(tm: Arc<SVMMemory>, svm: Arc<SVM>, aorb: u32) -> Result<SVMPrimitives, String> {
    let tx_body = TxBody {
        tx_hash: "".to_owned(),
        code_hash: "0xduangua".to_owned(),
        objs: vec!["0x1000001".to_owned(), "0x1000002".to_owned()],
        args: vec![SVMPrimitives::U24(1), SVMPrimitives::U24(6)],
    };

    process_tx(tx_body, tm, svm)
}
