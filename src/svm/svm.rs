use bend::{
    compile_book,
    diagnostics::{Diagnostics, DiagnosticsConfig},
    fun::{self, load_book::do_parse_book, Book, Term},
    readback_hvm_net, CompileOpts, CompileResult, RunOpts,
};
use builtins::{ADD_CODE, ADD_CODE_ID, SUB_CODE, SUB_CODE_ID};
use chrono::Utc;
use hvm::hvm::{GNet, TMem};
use log::info;
use std::{collections::HashMap, path::Path, sync::Arc};

use super::builtins;

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
        let ramdomized_hvm_out_path = format!("{}.out.hvm", Utc::now().timestamp_nanos());
        let run_opts = RunOpts {
            linear_readback: false,
            pretty: false,
            hvm_path: "hvm".to_string(),
            hvm_out_path: ramdomized_hvm_out_path,
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

        let out = Self::run_hvm(&core_book.build())?;
        let (net, stats) = parse_hvm_output(&out)?;
        let (term, diags) = readback_hvm_net(
            &net,
            &book,
            &labels,
            run_opts.linear_readback,
            compile_opts.adt_encoding,
        );

        Ok(Some((term, stats, diags)))
    }

    pub fn run_hvm(book: &hvm::hvm::Book) -> Result<String, String> {
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

        let mut result = "".to_string();

        // Parse the result
        if let Some(tree) = hvm::ast::Net::readback(&net, book) {
            result = format!("{}\n{}", result, format!("Result: {}", tree.show()));
        } else {
            result = format!(
                "{}\n{}",
                result,
                format!("Readback failed. Printing GNet memdump...\n")
            );
            result = format!("{}\n{}", result, format!("{}", net.show()));
        };

        // Prints interactions and time
        let itrs = net.itrs.load(std::sync::atomic::Ordering::Relaxed);
        result = format!("{}\n{}", result, format!("- ITRS: {}", itrs));
        result = format!(
            "{}\n{}",
            result,
            format!("- TIME: {:.2}s", duration.as_secs_f64())
        );
        result = format!(
            "{}\n{}",
            result,
            format!(
                "- MIPS: {:.2}",
                itrs as f64 / duration.as_secs_f64() / 1_000_000.0
            )
        );

        Ok(result)
    }
}

/// Reads the final output from HVM and separates the extra information.
fn parse_hvm_output(out: &str) -> Result<(hvm::ast::Net, String), String> {
    let Some((result, stats)) = out.split_once('\n') else {
        return Err(format!(
            "Failed to parse result from HVM (unterminated result).\nOutput from HVM was:\n{:?}",
            out
        ));
    };
    let mut p = hvm::ast::CoreParser::new(result);
    let Ok(net) = p.parse_net() else {
        return Err(format!(
            "Failed to parse result from HVM (invalid net).\nOutput from HVM was:\n{:?}",
            out
        ));
    };
    Ok((net, stats.to_string()))
}
