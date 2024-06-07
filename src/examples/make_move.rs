use crate::block_stm::svm_memory::{retry_transaction, SVMMemory};
use crate::svm::builtins::DUANGUA_CODE_ID;
use crate::svm::{primitive_types::SVMPrimitives, svm::SVM};
use std::sync::Arc;

pub async fn make_move(
    tm: Arc<SVMMemory>,
    svm: Arc<SVM>,
    aorb: u32,
) -> Result<SVMPrimitives, String> {
    let from_key = format!("0x0");
    let to_key = format!("0x1");
    let from_key_vec = from_key.clone().as_bytes().to_vec();
    let to_key_vec = to_key.clone().as_bytes().to_vec();
    match retry_transaction(tm, |txn| {
        let persona = match txn.read(from_key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", from_key)),
        };
        let personb = match txn.read(to_key_vec.clone()) {
            Some(value) => value,
            None => return Err(format!("key={} does not exist", to_key)),
        };

        let args = Some(vec![
            persona.to_term(),
            personb.to_term(),
            SVMPrimitives::U24(aorb).to_term(),
            SVMPrimitives::U24(1).to_term(),
        ]);
        match svm.clone().run_code(DUANGUA_CODE_ID, args) {
            Ok(Some((term, _stats, _diags))) => {
                // eprint!("i={} {diags}", i);
                // println!(
                //     "from_key={} Result:\n{}",
                //     from_key.clone(),
                //     term.display_pretty(0)
                // );
                let result = SVMPrimitives::from_term(term.clone());
                match result {
                    SVMPrimitives::Tup(ref els) => {
                        let (person_step, win) = (els[0].clone(), els[1].clone());
                        if aorb == 0 {
                            txn.write(from_key_vec.clone(), person_step);
                        } else {
                            txn.write(to_key_vec.clone(), person_step);
                        }
                        println!("{:?}", win);
                        return Ok(Some(win));
                    }
                    unknown => {
                        return Err(format!("unexpected type of result unknown={:?}", unknown))
                    }
                };
            }
            Ok(None) => return Err(format!("svm execution failed err=none result")),
            Err(e) => return Err(format!("svm execution failed err={}", e)),
        }
    }) {
        Ok(ret_val) => match ret_val {
            Some(ret_val) => return Ok(ret_val),
            None => return Err(format!("from_key={} err=must return", from_key.clone())),
        },
        Err(e) => {
            return Err(format!("from_key={} err={}", from_key.clone(), e));
        }
    }
}
