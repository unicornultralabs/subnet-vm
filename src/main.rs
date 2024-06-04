use svm::{
    builtins::{ADD_CODE, PARALLEL_HELLO_WORLD_CODE},
    primitive_types::SVMPrimitives,
};

pub mod svm;

fn main() {
    // if let Some((term, _stats, diags)) =
    //     svm::run_code(PARALLEL_HELLO_WORLD_CODE, None, None).expect("run code err")
    // {
    //     eprint!("{diags}");
    //     println!("Result:\n{}", term.display_pretty(0));
    // }

    let args = {
        let bal = SVMPrimitives::U24(0).to_term();
        let amt = SVMPrimitives::U24(100).to_term();
        Some(vec![bal, amt])
    };
    if let Some((term, _stats, diags)) =
        svm::run_code(ADD_CODE, Some("add"), args).expect("run code err")
    {
        eprint!("{diags}");
        println!("Result:\n{}", term.display_pretty(0));
    }
}
