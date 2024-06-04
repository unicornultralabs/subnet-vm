use block_stm::svm_memory::{retry_transaction, SVMMemory};
use svm::{builtins::ADD_CODE, primitive_types::SVMPrimitives};

pub mod block_stm;
pub mod executor;
pub mod svm;

fn main() {
    // if let Some((term, _stats, diags)) =
    //     svm::run_code(PARALLEL_HELLO_WORLD_CODE, None, None).expect("run code err")
    // {
    //     eprint!("{diags}");
    //     println!("Result:\n{}", term.display_pretty(0));
    // }

    // initially set value
    let key = "0x1999".as_bytes().to_vec();

    let tm = SVMMemory::new();
    retry_transaction(&tm, |txn| {
        txn.write(key.clone(), SVMPrimitives::U24(0));
    });

    // execute with vm and store back
    retry_transaction(&tm, |txn| {
        if let Some(value) = txn.read(key.clone()) {
            let args = {
                let amt = SVMPrimitives::U24(100).to_term();
                Some(vec![value.to_term(), amt])
            };

            match svm::run_code(ADD_CODE, Some("add"), args).expect("run code err") {
                Some((term, _stats, diags)) => {
                    eprint!("{diags}");
                    println!("Result:\n{}", term.display_pretty(0));
                    txn.write(key.clone(), SVMPrimitives::from_term(term));
                }
                None => {
                    eprint!("svm execution failed");
                }
            }
        }
    });
}
