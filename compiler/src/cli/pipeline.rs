use std::path::Path;
use std::process::{Command, ExitCode};

use super::report::write_stdout;
use super::{Emit, Options, Report};
use crate::diagnostic::{self, Diagnostic};
use crate::modules::{self, Graph};
use crate::source::SourceMap;
use crate::{backend, check, format as formatter, ir, lexer, lower, opt, parser, resolve};

pub struct Compiled {
    pub program: ir::Program,
    pub llvm: String,
}

pub fn check(path: &str, _source: &str, options: &Options) -> ExitCode {
    let loaded = modules::load(Path::new(path));
    let report = Report {
        sources: &loaded.graph.sources,
        json: options.json,
    };

    if !loaded.diagnostics.is_empty() {
        report.emit(&loaded.diagnostics);
        return ExitCode::FAILURE;
    }

    match compile(&loaded.graph, options) {
        Ok(_) => ExitCode::SUCCESS,
        Err(diagnostics) => {
            report.emit(&diagnostics);
            ExitCode::FAILURE
        }
    }
}

pub fn test(path: &str, _source: &str, options: &Options) -> ExitCode {
    let loaded = modules::load(Path::new(path));
    let sources = &loaded.graph.sources;
    let report = Report {
        sources,
        json: options.json,
    };

    if !loaded.diagnostics.is_empty() {
        report.emit(&loaded.diagnostics);
        return ExitCode::FAILURE;
    }

    let compiled = match compile(&loaded.graph, options) {
        Ok(compiled) => compiled,
        Err(diagnostics) => {
            report.emit(&diagnostics);
            return ExitCode::FAILURE;
        }
    };

    let found: Vec<(usize, String)> = compiled
        .program
        .functions
        .iter()
        .enumerate()
        .filter(|(index, function)| {
            compiled.program.entry_functions.contains(index)
                && function.name.starts_with("test_")
                && function.param_count == 0
        })
        .map(|(index, function)| (index, function.name.clone()))
        .collect();

    if found.is_empty() {
        report.note("no tests found: name a function `test_something` with no parameters");
        return ExitCode::FAILURE;
    }

    let indices: Vec<usize> = found.iter().map(|(index, _)| *index).collect();
    let llvm = backend::emit_tests(&compiled.program, sources, &indices);
    let binary = std::env::temp_dir().join(format!("dovik-test-{}", std::process::id()));
    let target = binary.display().to_string();

    if let Err(message) = link(&llvm, &target) {
        report.note(&message);
        return ExitCode::FAILURE;
    }

    let mut failed = 0;

    for (slot, (_, name)) in found.iter().enumerate() {
        let status = Command::new(&target).arg(slot.to_string()).status();

        let passed = matches!(status, Ok(code) if code.success());
        let mark = match passed {
            true => "ok  ",
            false => "FAIL",
        };

        println!("{mark} {name}");
        failed += usize::from(!passed);
    }

    let _ = std::fs::remove_file(&target);
    let total = found.len();
    println!();
    println!("{} passed, {failed} failed, {total} total", total - failed);

    match failed {
        0 => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

pub fn format(path: &str, source: &str, options: &Options) -> ExitCode {
    let sources = &SourceMap::single(path, source);
    let report = Report {
        sources,
        json: options.json,
    };

    let (tokens, lex_errors) = lexer::lex(sources.entry_text());

    if !lex_errors.is_empty() {
        report.emit(&diagnostic::from_lex(&lex_errors));
        return ExitCode::FAILURE;
    }

    let (_, parse_errors) = parser::parse(sources.entry_text(), &tokens);

    if !parse_errors.is_empty() {
        report.emit(&diagnostic::from_parse(&parse_errors));
        report.note("refusing to format a file that does not parse");
        return ExitCode::FAILURE;
    }

    let Some(formatted) = formatter::format(sources.entry_text()) else {
        report.note("could not format this file");
        return ExitCode::FAILURE;
    };

    if options.check_only {
        return match formatted == sources.entry_text() {
            true => ExitCode::SUCCESS,
            false => {
                report.note("file is not formatted");
                ExitCode::FAILURE
            }
        };
    }

    if options.write {
        if formatted == sources.entry_text() {
            return ExitCode::SUCCESS;
        }

        return match std::fs::write(path, &formatted) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                report.note(&format!("could not write: {err}"));
                ExitCode::FAILURE
            }
        };
    }

    write_stdout(formatted.lines().map(str::to_string));
    ExitCode::SUCCESS
}

pub fn build(path: &str, _source: &str, options: &Options) -> ExitCode {
    let loaded = modules::load(Path::new(path));
    let sources = &loaded.graph.sources;
    let report = Report {
        sources,
        json: options.json,
    };

    if !loaded.diagnostics.is_empty() {
        report.emit(&loaded.diagnostics);
        return ExitCode::FAILURE;
    }

    if options.emit == Emit::Tokens {
        let (tokens, errors) = lexer::lex(sources.text());

        if !errors.is_empty() {
            report.emit(&diagnostic::from_lex(&errors));
            return ExitCode::FAILURE;
        }

        write_stdout(tokens.iter().map(|token| {
            let (line, column) = sources.position(token.span.start);
            format!("{line}:{column}\t{:?}", token.kind)
        }));
        return ExitCode::SUCCESS;
    }

    let compiled = match compile(&loaded.graph, options) {
        Ok(compiled) => compiled,
        Err(diagnostics) => {
            report.emit(&diagnostics);
            return ExitCode::FAILURE;
        }
    };

    match options.emit {
        Emit::Ir => {
            write_stdout(compiled.program.to_string().lines().map(str::to_string));
            ExitCode::SUCCESS
        }
        Emit::Llvm => {
            write_stdout(compiled.llvm.lines().map(str::to_string));
            ExitCode::SUCCESS
        }
        _ => {
            let target = options
                .output
                .clone()
                .unwrap_or_else(|| default_output(path));

            match link(&compiled.llvm, &target) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    report.note(&message);
                    ExitCode::FAILURE
                }
            }
        }
    }
}

pub fn run(path: &str, _source: &str, options: &Options) -> ExitCode {
    let loaded = modules::load(Path::new(path));
    let sources = &loaded.graph.sources;
    let report = Report {
        sources,
        json: options.json,
    };

    if !loaded.diagnostics.is_empty() {
        report.emit(&loaded.diagnostics);
        return ExitCode::FAILURE;
    }

    let compiled = match compile(&loaded.graph, options) {
        Ok(compiled) => compiled,
        Err(diagnostics) => {
            report.emit(&diagnostics);
            return ExitCode::FAILURE;
        }
    };

    let target = std::env::temp_dir().join(format!("dovik-run-{}", std::process::id()));
    let target = target.display().to_string();

    if let Err(message) = link(&compiled.llvm, &target) {
        report.note(&message);
        return ExitCode::FAILURE;
    }

    let status = Command::new(&target).status();
    let _ = std::fs::remove_file(&target);

    match status {
        Ok(status) => match status.code() {
            Some(0) => ExitCode::SUCCESS,
            Some(_) => ExitCode::FAILURE,
            None => ExitCode::FAILURE,
        },
        Err(err) => {
            report.note(&format!("could not start the program: {err}"));
            ExitCode::FAILURE
        }
    }
}

pub fn compile(graph: &Graph, options: &Options) -> Result<Compiled, Vec<Diagnostic>> {
    let sources = &graph.sources;
    let (resolution, resolve_errors) = resolve::resolve(graph);

    if !resolve_errors.is_empty() {
        return Err(diagnostic::from_resolve(&resolve_errors));
    }

    let (typed, type_errors) = check::check(graph, &resolution);

    if !type_errors.is_empty() {
        return Err(diagnostic::from_type(&type_errors));
    }

    let mut program = lower::lower(graph, &resolution, &typed);

    if options.optimize {
        let fold_errors = opt::optimize(&mut program);

        if !fold_errors.is_empty() {
            return Err(diagnostic::from_type(&fold_errors));
        }
    }

    let llvm = backend::emit_with(&program, sources, !options.omit_checks);
    Ok(Compiled { program, llvm })
}

fn default_output(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "a.out".to_string())
}

fn link(llvm: &str, target: &str) -> Result<(), String> {
    let ll_path = format!("{target}.ll");

    if let Err(err) = std::fs::write(&ll_path, llvm) {
        return Err(format!("{ll_path}: {err}"));
    }

    let status = Command::new("clang")
        .args(["-O2", "-flto=full", "-o", target, &ll_path, &runtime_archive()])
        .arg("-lm")
        .status();

    match status {
        Ok(status) if status.success() => {
            let _ = std::fs::remove_file(&ll_path);
            Ok(())
        }
        Ok(_) => Err(format!("clang failed; the LLVM IR is at {ll_path}")),
        Err(err) => Err(format!("could not run clang: {err}")),
    }
}

pub fn runtime_archive() -> String {
    std::env::var("DOVIK_RUNTIME").unwrap_or_else(|_| env!("DOVIK_RUNTIME_OBJECT").to_string())
}
