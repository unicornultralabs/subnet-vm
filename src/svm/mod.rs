use bend::{
    diagnostics::{Diagnostics, DiagnosticsConfig},
    fun::{self, load_book::do_parse_book, Book, Name, Term},
    run_book, CompileOpts, RunOpts,
};
use std::path::Path;

pub mod builtins;
pub mod object;
pub mod primitive_types;

pub fn run_code(
    code: &str,
    // entrypoint: Option<&str>,
    arguments: Option<Vec<Term>>,
) -> Result<Option<(Term, String, Diagnostics)>, Diagnostics> {
    let load_book = || -> Result<Book, Diagnostics> {
        let builtins = fun::Book::builtins();
        let mut book = do_parse_book(code, Path::new(""), builtins)?;
        // book.entrypoint = entrypoint.map(Name::new);
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

    run_book(
        book,
        run_opts,
        compile_opts,
        diagnostics_cfg,
        arguments,
        run_cmd,
    )
}
