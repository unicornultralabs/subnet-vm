use std::path::Path;

use bend::{
    check_book, compile_book, desugar_book,
    diagnostics::{Diagnostics, DiagnosticsConfig, Severity},
    fun::{self, load_book::do_parse_book, Book, Name},
    hvm::display_hvm_book,
    load_file_to_book, run_book, AdtEncoding, CompileOpts, OptLevel, RunOpts,
};

fn main() {
    let load_book = || -> Result<Book, Diagnostics> {
        let code_path = Path::new("./code/parallel_hello_world.bend");
        let code = include_str!("./code/parallel_hello_world.bend");
        // let code_path = Path::new("./code/hello.bend");
        // let code = include_str!("./code/hello.bend");
        let builtins = fun::Book::builtins();
        let book = do_parse_book(&code, code_path, builtins)?;
        // println!("{book}");
        Ok(book)
    };

    let book = load_book().expect("lb failed");

    let run_opts = RunOpts {
        linear_readback: false,
        pretty: false,
        hvm_path: "hvm".to_owned(),
    };

    let compile_opts = CompileOpts {
        eta: true,
        prune: false,
        linearize_matches: bend::OptLevel::Enabled,
        float_combinators: true,
        merge: false,
        inline: false,
        check_net_size: false,
        adt_encoding: bend::AdtEncoding::NumScott,
    };
    let diagnostics_cfg = DiagnosticsConfig {
        verbose: false,
        irrefutable_match: bend::diagnostics::Severity::Allow,
        redundant_match: bend::diagnostics::Severity::Allow,
        unreachable_match: bend::diagnostics::Severity::Allow,
        unused_definition: bend::diagnostics::Severity::Allow,
        repeated_bind: bend::diagnostics::Severity::Allow,
        recursion_cycle: bend::diagnostics::Severity::Allow,
    };
    let run_cmd = "run";

    // let arguments = Some(vec![]);
    let arguments = None;

    if let Some((term, stats, diags)) = run_book(
        book,
        run_opts,
        compile_opts,
        diagnostics_cfg,
        arguments,
        run_cmd,
    )
    .expect("rb failed")
    {
        eprint!("{diags}");
        println!("Result:\n{}", term.display_pretty(0));
    }

    println!("Hello, world!");
}
