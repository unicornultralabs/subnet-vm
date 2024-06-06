use super::builtins::{self, DUANGUA_CODE, DUANGUA_CODE_ID, TRANSFER_CODE, TRANSFER_CODE_ID};
use bend::{
    compile_book,
    diagnostics::{Diagnostics, DiagnosticsConfig},
    fun::{self, load_book::do_parse_book, Book, Term},
    readback_hvm_net, CompileOpts, CompileResult, RunOpts,
};
use builtins::{ADD_CODE, ADD_CODE_ID, SUB_CODE, SUB_CODE_ID};
use hvm::hvm::{GNet, TMem};
use std::{collections::HashMap, path::Path, sync::Arc};

pub struct SVM {
    books: Arc<HashMap<String, Book>>,
}

impl SVM {
    pub fn new() -> Self {
        let mut books = HashMap::new();

        let builtins = fun::Book::builtins();
        let codes = vec![
            (ADD_CODE_ID, ADD_CODE),
            (SUB_CODE_ID, SUB_CODE),
            (TRANSFER_CODE_ID, TRANSFER_CODE),
            (DUANGUA_CODE_ID, DUANGUA_CODE),
        ];
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
        // TODO(rameight): HVM2 doesn't enable entrypoint running yet.
        // entrypoint: Option<&str>,
        arguments: Option<Vec<Term>>,
    ) -> Result<Option<(Term, String, Diagnostics)>, Diagnostics> {
        let book = self.books.get(code_id).expect("load book failed").clone();
        let run_opts = RunOpts {
            linear_readback: false,
            pretty: false,
            hvm_path: "hvm".to_string(),
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
        self.run_book(book, run_opts, compile_opts, diagnostics_cfg, arguments)

        // TODO(rameight): by calling the hvm binary, it does not work as expected
        // since it fails to streamlining the VM result
        // run_book(
        //     book,
        //     run_opts,
        //     compile_opts,
        //     diagnostics_cfg,
        //     arguments,
        //     "run",
        // )
    }

    fn run_book(
        self: Arc<Self>,
        mut book: Book,
        run_opts: RunOpts,
        compile_opts: CompileOpts,
        diagnostics_cfg: DiagnosticsConfig,
        args: Option<Vec<Term>>,
    ) -> Result<Option<(Term, String, Diagnostics)>, Diagnostics> {
        let CompileResult {
            hvm_book: core_book,
            labels,
            diagnostics,
        } = compile_book(&mut book, compile_opts.clone(), diagnostics_cfg, args)?;
        eprint!("{diagnostics}");

        let (net, stats) = Self::run_hvm(&core_book.build())?;
        let (term, diags) = readback_hvm_net(
            &net,
            &book,
            &labels,
            run_opts.linear_readback,
            compile_opts.adt_encoding,
        );

        Ok(Some((term, stats, diags)))
    }

    pub fn run_hvm(book: &hvm::hvm::Book) -> Result<(hvm::ast::Net, String), String> {
        // Initializes the global net
        let net = GNet::new(1 << 29, 1 << 29);

        // Initializes threads
        let mut tm = TMem::new(0, 1);

        // Creates an initial redex that calls main
        let main_id = book.defs.iter().position(|def| def.name == "main").unwrap();
        tm.rbag.push_redex(hvm::hvm::Pair::new(
            hvm::hvm::Port::new(hvm::hvm::REF, main_id as u32),
            hvm::hvm::ROOT,
        ));
        net.vars_create(hvm::hvm::ROOT.get_val() as usize, hvm::hvm::NONE);

        // Starts the timer
        let start = std::time::Instant::now();

        // Evaluates
        tm.evaluator(&net, &book);

        // Stops the timer
        let duration = start.elapsed();

        // Prints interactions and time
        let stats = {
            let itrs = net.itrs.load(std::sync::atomic::Ordering::Relaxed);
            format!(
                r#"- ITRS: {}
- TIME: {:.2}s
- MIPS: {:.2}"#,
                itrs,
                duration.as_secs_f64(),
                itrs as f64 / duration.as_secs_f64() / 1_000_000.0
            )
        };

        // Parse the result
        let result = if let Some(tree) = hvm::ast::Net::readback(&net, book) {
            format!("{}", tree.show())
        } else {
            format!(
                r#"Readback failed. Printing GNet memdump...
{}"#,
                net.show()
            )
        };

        let mut p = hvm::ast::CoreParser::new(&result);
        let Ok(net) = p.parse_net() else {
            return Err(format!(
                "Failed to parse result from HVM (invalid net).\nOutput from HVM was:\n{:?}",
                format!(
                    r#"{}
{}"#,
                    result, stats
                )
            ));
        };
        Ok((net, stats))
    }
}
