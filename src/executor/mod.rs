use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use crate::svm::{primitive_types::SVMPrimitives, svm::SVM};
use bend::fun::Term;
use log::error;
use redis::{DPNRedisKey, RedisService};
use std::sync::Arc;
use types::TxBody;

pub mod types;
pub mod redis;

pub fn process_tx(
    tx_body: TxBody,
    tm: Arc<SVMMemory>,
    svm: Arc<SVM>,
) -> Result<SVMPrimitives, std::string::String> {
    let tm = tm.clone();
    let svm = svm.clone();

    let redis_svc = Arc::new(RedisService::new("redis://:dpn@localhost:6379".to_string()).unwrap());

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
                            if let Ok(svm_primitive_json) = serde_json::to_string(&modified_objs[index].clone()) {
                                let (k, f) = DPNRedisKey::get_vm_kf(obj_hash.as_bytes().to_vec());
                                if let Err(e) = Arc::clone(&redis_svc).hset(k, f, svm_primitive_json) {
                                    error!("failed to set svm_primitive_json err={}", e)
                                };
                            } else {
                                error!("failed when parse svm primitive to string");
                            }
                        }
                        return Ok(result);
                    }
                    _ => return Err("unexpected type of result".to_string()),
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
