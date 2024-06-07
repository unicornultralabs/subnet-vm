use crate::svm::primitive_types::SVMPrimitives;
use std::sync::Arc;
use svm_memory::{retry_transaction, SVMMemory};

pub mod svm_memory;

pub fn get_val(tm: Arc<SVMMemory>, key: String) -> Option<SVMPrimitives> {
    let key_vec = key.clone().as_bytes().to_vec();
    match retry_transaction(tm.clone(), |txn| {
        let return_value = match txn.read(key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", key)),
        };
        Ok(return_value)
    }) {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}
