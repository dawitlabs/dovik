use std::io::{self, Write};
use std::process::ExitCode;

use compiler::error::line_col;
use compiler::{lexer, parser};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut dump_tokens = false;
    let mut path = None;

    for arg in args.by_ref() {
        match arg.as_str() {
            "--tokens" => dump_tokens = true,
            _ => path = Some(arg),
        }
    }

    let Some(path) = path else {
        eprintln!("usage: compiler [--tokens] <file.dovk>");
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
        let mut out = io::stdout().lock();
        for token in &tokens {
            let (line, column) = line_col(&source, token.span.start);
            // A closed pipe (`| head`) is not an error worth panicking over.
            if writeln!(out, "{line}:{column}\t{:?}", token.kind).is_err() {
                break;
            }
        }
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

    if !dump_tokens {
        println!("{program:#?}");
    }

    ExitCode::SUCCESS
}

fn report(path: &str, source: &str, offset: usize, message: &dyn std::fmt::Display) {
    let (line, column) = line_col(source, offset);
    eprintln!("{path}:{line}:{column}: error: {message}");
}
