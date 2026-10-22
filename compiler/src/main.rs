use std::io::{self, Write};
use std::process::ExitCode;

use compiler::error::line_col;
use compiler::error::line_col as position;
use compiler::{backend, check, interp, lexer, lower, opt, parser, resolve};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut dump_tokens = false;
    let mut dump_ir = false;
    let mut execute = false;
    let mut optimize = true;
    let mut dump_llvm = false;
    let mut output = None;
    let mut path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--tokens" => dump_tokens = true,
            "--ir" => dump_ir = true,
            "--run" => execute = true,
            "--no-opt" => optimize = false,
            "--emit-llvm" => dump_llvm = true,
            "-o" => output = args.next(),
            _ => path = Some(arg),
        }
    }

    let Some(path) = path else {
        eprintln!(
            "usage: compiler [--tokens] [--ir] [--emit-llvm] [--run] [--no-opt] [-o out] <file.dovk>"
        );
        return ExitCode::FAILURE;
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    let (tokens, lex_errors) = lexer::lex(&source);

    if dump_tokens {
        let lines = tokens.iter().map(|token| {
            let (line, column) = line_col(&source, token.span.start);
            format!("{line}:{column}\t{:?}", token.kind)
        });
        write_lines(lines);
    }

    if !lex_errors.is_empty() {
        for error in &lex_errors {
            report(&path, &source, error.span.start, &error.kind);
        }
        return ExitCode::FAILURE;
    }

    let (program, parse_errors) = parser::parse(&source, &tokens);

    for error in &parse_errors {
        report(&path, &source, error.span.start, &error.kind);
    }

    if !parse_errors.is_empty() {
        return ExitCode::FAILURE;
    }

    let (resolution, resolve_errors) = resolve::resolve(&program);

    for error in &resolve_errors {
        report(&path, &source, error.span.start, &error.kind);
    }

    if !resolve_errors.is_empty() {
        return ExitCode::FAILURE;
    }

    let (typed, type_errors) = check::check(&program, &resolution);

    for error in &type_errors {
        report(&path, &source, error.span.start, &error.kind);
    }

    if !type_errors.is_empty() {
        return ExitCode::FAILURE;
    }

    let mut ir = lower::lower(&program, &resolution, &typed);

    if optimize {
        let fold_errors = opt::optimize(&mut ir);

        for error in &fold_errors {
            report(&path, &source, error.span.start, &error.kind);
        }

        if !fold_errors.is_empty() {
            return ExitCode::FAILURE;
        }
    }

    if execute {
        let run = interp::run(&ir);
        write_lines(run.output.into_iter());

        return match run.outcome {
            interp::Outcome::Finished => ExitCode::SUCCESS,
            interp::Outcome::Trapped(trap) => {
                let (line, column) = position(&source, trap.span.start);
                eprintln!("{path}:{line}:{column}: runtime error: {}", trap.error);
                ExitCode::FAILURE
            }
            interp::Outcome::LimitReached => {
                eprintln!("{path}: execution limit reached");
                ExitCode::FAILURE
            }
        };
    }

    if dump_llvm {
        write_lines(backend::emit(&ir, &source).lines().map(str::to_string));
        return ExitCode::SUCCESS;
    }

    if let Some(target) = output {
        return build(&ir, &source, &target);
    }

    if dump_ir {
        write_lines(ir.to_string().lines().map(str::to_string));
    } else if !dump_tokens {
        write_lines(format!("{program:#?}").lines().map(str::to_string));
    }

    ExitCode::SUCCESS
}

fn build(ir: &compiler::ir::Program, source: &str, target: &str) -> ExitCode {
    let llvm = backend::emit(ir, source);
    let ll_path = format!("{target}.ll");

    if let Err(err) = std::fs::write(&ll_path, &llvm) {
        eprintln!("{ll_path}: {err}");
        return ExitCode::FAILURE;
    }

    let archive = runtime_archive();

    let status = std::process::Command::new("clang")
        .args(["-O2", "-o", target, &ll_path, &archive, "-lm", "-lpthread", "-ldl"])
        .status();

    match status {
        Ok(status) if status.success() => {
            let _ = std::fs::remove_file(&ll_path);
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("clang failed; the LLVM IR is at {ll_path}");
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("could not run clang: {err}");
            ExitCode::FAILURE
        }
    }
}

fn runtime_archive() -> String {
    if let Ok(path) = std::env::var("DOVIK_RUNTIME") {
        return path;
    }

    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("libdovik_rt.a")))
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "libdovik_rt.a".to_string())
}

fn write_lines(lines: impl Iterator<Item = String>) {
    let mut out = io::stdout().lock();

    for line in lines {
        if writeln!(out, "{line}").is_err() {
            return;
        }
    }
}

fn report(path: &str, source: &str, offset: usize, message: &dyn std::fmt::Display) {
    let (line, column) = line_col(source, offset);
    eprintln!("{path}:{line}:{column}: error: {message}");
}
