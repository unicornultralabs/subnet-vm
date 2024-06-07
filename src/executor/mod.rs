use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use crate::svm::{primitive_types::SVMPrimitives, svm::SVM};
use bend::fun::Term;
use log::info;
use std::sync::Arc;
use types::TxBody;

pub mod types;

pub fn process_tx(
    tx_body: TxBody,
    tm: Arc<SVMMemory>,
    svm: Arc<SVM>,
) -> Result<SVMPrimitives, std::string::String> {
    let tm = tm.clone();
    let svm = svm.clone();

    let result = retry_transaction(tm, |txn| {
        let mut objects = vec![];
        for obj_hash in tx_body.objs.clone() {
            let object = match txn.read(obj_hash.as_bytes().to_vec()) {
                Some(object) => object,
                None => return Err(format!("key={} does not exist", obj_hash)),
            };
            objects.push(object)
        }
        let mut args = objects;
        args.extend_from_slice(&tx_body.args);

        // due to limitations of HVM, we cannot read data from this code
        // however, we can feed the data from arguments
        // so arguments of main is the thing we want to modify PLUS the actual arguments.
        let args: Vec<Term> = args.iter().map(|arg| arg.to_term()).collect();
        match svm.clone().run_code(&tx_body.code_hash, Some(args)) {
            Ok((term, _stats, _diags)) => {
                let result = SVMPrimitives::from_term(term.clone());
                match result {
                    SVMPrimitives::Tup(ref els) => {
                        // VM always returned the (un)modified objects as in the order
                        // of receiving in input. We write back to SVMMemmory.
                        let modified_objs = els.clone();
                        for (index, obj_hash) in tx_body.objs.iter().enumerate() {
                            txn.write(obj_hash.as_bytes().to_vec(), modified_objs[index].clone());
                        }
                        return Ok(result);
                    }
                    _ => return Err(format!("unexpected type of result term={:#?}", term)),
                };
            }
            Err(e) => Err(format!("svm execution failed err={}", e)),
        }
    });

    match result {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
}
