use std::path::PathBuf;
use std::process::Command;

use compiler::error::TypeErrorKind::{self, *};
use compiler::ir::RuntimeError;
use compiler::types::Type;
use compiler::{backend, check, interp, ir, lexer, lower, opt, parser, resolve};

fn lowered(source: &str) -> ir::Program {
    let (tokens, lex_errors) = lexer::lex(source);
    assert_eq!(lex_errors, [], "lex errors");
    let (ast, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let (resolution, resolve_errors) = resolve::resolve(&ast);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    let (typed, type_errors) = check::check(&ast, &resolution);
    assert!(type_errors.is_empty(), "type errors: {type_errors:?}");
    let mut program = lower::lower(&ast, &resolution, &typed);
    assert!(opt::optimize(&mut program).is_empty());
    program
}

fn errors(source: &str) -> Vec<TypeErrorKind> {
    let (tokens, _) = lexer::lex(source);
    let (ast, parse_errors) = parser::parse(source, &tokens);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let (resolution, resolve_errors) = resolve::resolve(&ast);
    assert!(resolve_errors.is_empty(), "resolve errors: {resolve_errors:?}");
    check::check(&ast, &resolution)
        .1
        .into_iter()
        .map(|error| error.kind)
        .collect()
}

fn sandbox(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("dovik-io-tests").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn output(source: &str) -> Vec<String> {
    let run = interp::run(&lowered(source));
    assert_eq!(run.outcome, interp::Outcome::Finished, "{run:?}");
    run.output
}

fn printed(source: &str) -> String {
    let run = interp::run(&lowered(source));
    assert_eq!(run.outcome, interp::Outcome::Finished, "{run:?}");
    run.output
        .iter()
        .map(|line| format!("{line}\n"))
        .collect()
}

fn output_with(source: &str, arguments: &[&str]) -> Vec<String> {
    let args = arguments.iter().map(|a| a.to_string()).collect();
    let run = interp::run_with_arguments(&lowered(source), args);
    assert_eq!(run.outcome, interp::Outcome::Finished, "{run:?}");
    run.output
}

fn trap(source: &str) -> RuntimeError {
    interp::run(&lowered(source))
        .trap()
        .expect("expected a trap")
        .error
}

#[test]
fn write_then_read_round_trips() {
    let dir = sandbox("round_trip");
    let path = dir.join("note.txt").display().to_string();
    let source = format!(
        "p = \"{path}\"\nwrite_file(p, \"hello\\n\")\nprint(read_file(p))\nprint(len(read_file(p)))\n"
    );

    assert_eq!(printed(&source), "hello\n\n6\n");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "hello\n");
}

#[test]
fn writing_truncates_and_appending_does_not() {
    let dir = sandbox("append");
    let path = dir.join("log.txt").display().to_string();
    let source = format!(
        "p = \"{path}\"\nwrite_file(p, \"a\")\nwrite_file(p, \"b\")\nappend_file(p, \"c\")\nprint(read_file(p))\n"
    );

    assert_eq!(output(&source), ["bc"]);
}

#[test]
fn appending_to_a_missing_file_creates_it() {
    let dir = sandbox("create");
    let path = dir.join("fresh.txt").display().to_string();
    let source = format!("p = \"{path}\"\nappend_file(p, \"x\")\nprint(read_file(p))\n");

    assert_eq!(output(&source), ["x"]);
}

#[test]
fn file_exists_reports_both_ways() {
    let dir = sandbox("exists");
    let path = dir.join("there.txt").display().to_string();
    let missing = dir.join("absent.txt").display().to_string();
    let source = format!(
        "write_file(\"{path}\", \"\")\nprint(file_exists(\"{path}\"))\nprint(file_exists(\"{missing}\"))\n"
    );

    assert_eq!(output(&source), ["true", "false"]);
}

#[test]
fn reading_a_missing_file_traps() {
    let dir = sandbox("missing");
    let path = dir.join("nope.txt").display().to_string();
    assert_eq!(
        trap(&format!("print(read_file(\"{path}\"))\n")),
        RuntimeError::InputOutput
    );
}

#[test]
fn writing_to_an_unwritable_path_traps() {
    let source = "write_file(\"/proc/definitely-not-writable/x\", \"x\")\n";
    assert_eq!(trap(source), RuntimeError::InputOutput);
}

#[test]
fn file_exists_lets_a_program_avoid_the_trap() {
    let dir = sandbox("guarded");
    let path = dir.join("nope.txt").display().to_string();
    let source = format!(
        "p = \"{path}\"\nif file_exists(p):\n    print(read_file(p))\nelse:\n    print(\"missing\")\n"
    );

    assert_eq!(output(&source), ["missing"]);
}

#[test]
fn arguments_are_readable() {
    let source = "\
count = argument_count()
print(count)
i = 0
while i < count:
    print(argument(i))
    i = i + 1
";
    assert_eq!(output_with(source, &["one", "two"]), ["2", "one", "two"]);
    assert_eq!(output_with(source, &[]), ["0"]);
}

#[test]
fn an_out_of_range_argument_traps() {
    assert_eq!(
        trap("print(argument(0))\n"),
        RuntimeError::IndexOutOfBounds
    );
}

#[test]
fn io_argument_types_are_enforced() {
    assert_eq!(
        errors("print(read_file(1))\n"),
        [BuiltinArgument { name: "read_file", found: Type::Int }]
    );
    assert_eq!(
        errors("write_file(\"p\", 1)\n"),
        [ArgumentType {
            name: "write_file".into(),
            position: 2,
            expected: Type::Str,
            found: Type::Int
        }]
    );
    assert_eq!(
        errors("print(argument(\"0\"))\n"),
        [BuiltinArgument { name: "argument", found: Type::Str }]
    );
    assert_eq!(
        errors("print(argument_count(1))\n"),
        [ArgumentCount { name: "argument_count".into(), expected: 0, found: 1 }]
    );
}

#[test]
fn write_file_returns_unit() {
    assert_eq!(errors("x = write_file(\"p\", \"c\")\n"), [UnitAsValue]);
}

#[test]
fn native_matches_the_interpreter_for_io() {
    let dir = sandbox("native");
    let path = dir.join("shared.txt").display().to_string();
    let source = format!(
        "p = \"{path}\"\nwrite_file(p, \"alpha\\n\")\nappend_file(p, \"beta\\n\")\nprint(file_exists(p))\nprint(len(read_file(p)))\nfor line in read_file(p):\n    print(line)\n"
    );

    let expected = interp::run(&lowered(&source));
    assert_eq!(expected.outcome, interp::Outcome::Finished, "{expected:?}");

    let program = lowered(&source);
    let ll = dir.join("prog.ll");
    let binary = dir.join("prog");
    std::fs::write(&ll, backend::emit(&program, &source)).expect("write ir");

    let build = Command::new("clang")
        .args(["-O2", "-flto=full"])
        .arg("-o")
        .arg(&binary)
        .arg(&ll)
        .arg(env!("DOVIK_RUNTIME_OBJECT"))
        .arg("-lm")
        .output()
        .expect("clang");
    assert!(
        build.status.success(),
        "clang rejected it:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&binary)
        .env("DOVIK_CHECK_LEAKS", "1")
        .output()
        .expect("run");
    let native = String::from_utf8_lossy(&run.stdout).to_string();
    let interpreted: String = expected
        .output
        .iter()
        .map(|line| format!("{line}\n"))
        .collect();

    assert_eq!(native, interpreted, "native differs from the interpreter");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("0 live object"),
        "leaked: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn list_dir_reports_sorted_names() {
    let dir = sandbox("listing");
    let path = dir.display().to_string();
    std::fs::write(dir.join("b.txt"), "").expect("write");
    std::fs::write(dir.join("a.txt"), "").expect("write");
    std::fs::create_dir(dir.join("inner")).expect("mkdir");

    let source = format!("print(join(list_dir(\"{path}\"), \",\"))\n");
    assert_eq!(output(&source), ["a.txt,b.txt,inner"]);
}

#[test]
fn list_dir_on_an_empty_directory_is_empty() {
    let dir = sandbox("empty_listing");
    let path = dir.display().to_string();
    let source = format!("print(len(list_dir(\"{path}\")))\n");
    assert_eq!(output(&source), ["0"]);
}

#[test]
fn is_dir_separates_directories_from_files() {
    let dir = sandbox("kinds");
    let path = dir.display().to_string();
    let file = dir.join("f.txt");
    std::fs::write(&file, "").expect("write");
    let file = file.display().to_string();
    let missing = dir.join("absent").display().to_string();

    let source = format!(
        "print(is_dir(\"{path}\"))\nprint(is_dir(\"{file}\"))\nprint(is_dir(\"{missing}\"))\n"
    );
    assert_eq!(output(&source), ["true", "false", "false"]);
}

#[test]
fn directories_round_trip() {
    let dir = sandbox("round");
    let made = dir.join("made").display().to_string();
    let source = format!(
        "create_dir(\"{made}\")\nprint(is_dir(\"{made}\"))\nremove_dir(\"{made}\")\nprint(is_dir(\"{made}\"))\n"
    );
    assert_eq!(output(&source), ["true", "false"]);
}

#[test]
fn remove_file_deletes_one_file() {
    let dir = sandbox("deletion");
    let path = dir.join("gone.txt");
    std::fs::write(&path, "x").expect("write");
    let path = path.display().to_string();

    let source = format!(
        "print(file_exists(\"{path}\"))\nremove_file(\"{path}\")\nprint(file_exists(\"{path}\"))\n"
    );
    assert_eq!(output(&source), ["true", "false"]);
}

#[test]
fn directory_failures_trap() {
    let dir = sandbox("failures");
    let missing = dir.join("absent").display().to_string();
    let existing = dir.display().to_string();
    let occupied = dir.join("taken.txt");
    std::fs::write(&occupied, "").expect("write");
    let occupied = occupied.display().to_string();

    assert_eq!(
        trap(&format!("print(len(list_dir(\"{missing}\")))\n")),
        RuntimeError::InputOutput
    );
    assert_eq!(
        trap(&format!("create_dir(\"{existing}\")\n")),
        RuntimeError::InputOutput
    );
    assert_eq!(
        trap(&format!("remove_dir(\"{missing}\")\n")),
        RuntimeError::InputOutput
    );
    assert_eq!(
        trap(&format!("remove_file(\"{existing}\")\n")),
        RuntimeError::InputOutput
    );
    assert_eq!(
        trap(&format!("remove_dir(\"{occupied}\")\n")),
        RuntimeError::InputOutput
    );
}

#[test]
fn directory_builtins_reject_wrong_types() {
    assert_eq!(
        errors("print(is_dir(1))\n"),
        [BuiltinArgument { name: "is_dir", found: Type::Int }]
    );
    assert_eq!(errors("x = create_dir(\"d\")\n"), [UnitAsValue]);
}

#[test]
fn native_matches_the_interpreter_for_directories() {
    let dir = sandbox("native_dirs");
    let root = dir.display().to_string();
    let source = format!(
        "create_dir(\"{root}/nest\")\nwrite_file(\"{root}/nest/b\", \"\")\nwrite_file(\"{root}/nest/a\", \"\")\nprint(join(list_dir(\"{root}/nest\"), \",\"))\nprint(is_dir(\"{root}/nest\"))\nremove_file(\"{root}/nest/a\")\nremove_file(\"{root}/nest/b\")\nremove_dir(\"{root}/nest\")\nprint(is_dir(\"{root}/nest\"))\n"
    );

    let program = lowered(&source);
    let ll = dir.join("dirs.ll");
    let binary = dir.join("dirs");
    std::fs::write(&ll, backend::emit(&program, &source)).expect("write ir");

    let build = Command::new("clang")
        .args(["-O2", "-flto=full"])
        .arg("-o")
        .arg(&binary)
        .arg(&ll)
        .arg(env!("DOVIK_RUNTIME_OBJECT"))
        .arg("-lm")
        .output()
        .expect("clang");
    assert!(
        build.status.success(),
        "clang rejected it:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&binary)
        .env("DOVIK_CHECK_LEAKS", "1")
        .output()
        .expect("run");

    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "a,b\ntrue\nfalse\n",
        "native differs: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("0 live object"),
        "leaked: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}
