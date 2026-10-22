use std::io::{self, Write};
use std::process::ExitCode;

use compiler::error::line_col;
use compiler::lexer;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: compiler <file.dovk>");
        return ExitCode::FAILURE;
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    let (tokens, errors) = lexer::lex(&source);

    let mut out = io::stdout().lock();
    for token in &tokens {
        let (line, column) = line_col(&source, token.span.start);
        // A closed pipe (`| head`) is not an error worth panicking over.
        if writeln!(out, "{line}:{column}\t{:?}", token.kind).is_err() {
            break;
        }
    }
    drop(out);

    for error in &errors {
        let (line, column) = line_col(&source, error.span.start);
        eprintln!("{path}:{line}:{column}: error: {}", error.kind);
    }

    if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
