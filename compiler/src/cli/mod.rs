mod pipeline;
mod report;

use std::process::ExitCode;

use crate::target::Target;

pub use pipeline::{
    compile, sanitized_runtime_archive, wasi_sysroot, wasm_runtime_archive, Compiled,
};
pub use report::Report;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
dovik — the Dovik compiler

usage:
  dovik build <file.dovk> [-o <out>] [--target <name>] [options]
  dovik run <file.dovk> [options] [-- <args for the program>]
  dovik check <file.dovk> [options]
  dovik format <file.dovk> [--check] [--write]
  dovik test <file.dovk>
  dovik --version
  dovik --help

options:
  -o <path>       where to write the executable (default: the file's stem)
  --json          emit diagnostics as one JSON object per line
  --emit-llvm     write LLVM IR to stdout instead of an executable
  --emit-ir       write the Dovik IR to stdout instead of an executable
  --emit-tokens   write the token stream to stdout instead of an executable
  --no-opt        skip optimization
  --write         rewrite the file in place (format only)
  --check         exit 1 if the file is not already formatted (format only)
  --              stop reading options; the rest goes to the program (run only)
  --target <name> native (the default) or wasm. A wasm build writes a
                  WebAssembly module and needs wasi-libc; run it with a WASI
                  host such as `node --experimental-wasi-unstable-preview1`
  --depth-guard   check recursion depth in the program instead of relying on a
                  signal handler. A wasm build turns this on by itself, because
                  WebAssembly has no signals (DECISIONS.md D030)

  --omit-runtime-checks-for-benchmarking
                  build without the overflow, zero-divisor and conversion
                  checks. The result can produce wrong answers instead of
                  failing. It exists only to price those checks against a
                  build that lacks them (BENCHMARKS.md section 4).
";

pub struct Options {
    pub json: bool,
    pub optimize: bool,
    pub output: Option<String>,
    pub emit: Emit,
    pub write: bool,
    pub check_only: bool,
    pub omit_checks: bool,
    pub depth_guard: bool,
    pub target: Target,
    pub program_args: Vec<String>,
}

#[derive(PartialEq, Eq)]
pub enum Emit {
    Executable,
    Llvm,
    Ir,
    Tokens,
}

pub fn main(args: Vec<String>) -> ExitCode {
    let Some((command, rest)) = args.split_first() else {
        eprint!("{USAGE}");
        return ExitCode::FAILURE;
    };

    match command.as_str() {
        "--version" | "-V" | "version" => {
            println!("dovik {VERSION}");
            ExitCode::SUCCESS
        }
        "--help" | "-h" | "help" => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        "build" => with_file(rest, pipeline::build),
        "run" => with_file(rest, pipeline::run),
        "check" => with_file(rest, pipeline::check),
        "format" => with_file(rest, pipeline::format),
        "test" => with_file(rest, pipeline::test),
        other => {
            eprintln!("unknown command `{other}`\n");
            eprint!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn with_file(
    args: &[String],
    action: impl Fn(&str, &str, &Options) -> ExitCode,
) -> ExitCode {
    let mut options = Options {
        json: false,
        optimize: true,
        output: None,
        emit: Emit::Executable,
        write: false,
        check_only: false,
        omit_checks: false,
        depth_guard: false,
        target: Target::Native,
        program_args: Vec::new(),
    };
    let mut path = None;
    let mut pending = args.iter();

    while let Some(arg) = pending.next() {
        match arg.as_str() {
            "--json" => options.json = true,
            "--no-opt" => options.optimize = false,
            "--emit-llvm" => options.emit = Emit::Llvm,
            "--emit-ir" => options.emit = Emit::Ir,
            "--emit-tokens" => options.emit = Emit::Tokens,
            "--write" => options.write = true,
            "--check" => options.check_only = true,
            "--omit-runtime-checks-for-benchmarking" => options.omit_checks = true,
            "--depth-guard" => options.depth_guard = true,
            "--target" => {
                let Some(name) = pending.next() else {
                    eprintln!("`--target` needs a name: native or wasm");
                    return ExitCode::FAILURE;
                };

                match Target::parse(name) {
                    Some(target) => options.target = target,
                    None => {
                        eprintln!("unknown target `{name}`; try native or wasm");
                        return ExitCode::FAILURE;
                    }
                }
            }
            "-o" => options.output = pending.next().cloned(),
            "--" => {
                options.program_args = pending.cloned().collect();
                break;
            }
            other if other.starts_with('-') => {
                eprintln!("unknown option `{other}`");
                return ExitCode::FAILURE;
            }
            other if path.is_none() => path = Some(other.to_string()),
            other => {
                eprintln!("unexpected argument `{other}`; pass program arguments after `--`");
                return ExitCode::FAILURE;
            }
        }
    }

    let Some(path) = path else {
        eprintln!("expected a .dovk file\n");
        eprint!("{USAGE}");
        return ExitCode::FAILURE;
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    action(&path, &source, &options)
}
