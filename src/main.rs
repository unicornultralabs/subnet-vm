use svm::builtins::PARALLEL_HELLO_WORLD_CODE;

pub mod svm;

fn main() {
    if let Some((term, _stats, diags)) =
        svm::run_code(PARALLEL_HELLO_WORLD_CODE, None).expect("run code err")
    {
        eprint!("{diags}");
        println!("Result:\n{}", term.display_pretty(0));
    }
}
