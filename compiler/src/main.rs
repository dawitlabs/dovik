use std::process::ExitCode;

fn main() -> ExitCode {
    compiler::cli::main(std::env::args().skip(1).collect())
}
