use bend::{
    diagnostics::{Diagnostics, DiagnosticsConfig},
    fun::{self, load_book::do_parse_book, Book, Term},
    run_book, CompileOpts, RunOpts,
};
use builtins::{ADD_CODE, ADD_CODE_ID, SUB_CODE, SUB_CODE_ID};
use chrono::Utc;
use std::{collections::HashMap, path::Path, sync::Arc};

pub mod builtins;
pub mod object;
pub mod primitive_types;

pub struct SVM {
    books: Arc<HashMap<String, Book>>,
}

impl SVM {
    pub fn new() -> Self {
        let mut books = HashMap::new();

        let builtins = fun::Book::builtins();
        let codes = vec![(ADD_CODE_ID, ADD_CODE), (SUB_CODE_ID, SUB_CODE)];
        for code in codes {
            let builtins = builtins.clone();
            let book = do_parse_book(code.1, Path::new(""), builtins).expect("lb failed");
            books.insert(code.0.to_string(), book);
        }

        Self {
            books: Arc::new(books),
        }
    }

    pub fn run_code(
        self: Arc<Self>,
        code_id: &str,
        // entrypoint: Option<&str>,
        arguments: Option<Vec<Term>>,
    ) -> Result<Option<(Term, String, Diagnostics)>, Diagnostics> {
        let book = self.books.get(code_id).expect("load book failed").clone();
        let ramdomized_hvm_path = format!("hvm-{}", Utc::now().timestamp_nanos());
        let run_opts = RunOpts {
            linear_readback: false,
            pretty: false,
            hvm_path: ramdomized_hvm_path,
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
}
