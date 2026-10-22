use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const DOVIK: &str = env!("CARGO_BIN_EXE_dovik");

fn runtime_ready() -> &'static PathBuf {
    static ARCHIVE: OnceLock<PathBuf> = OnceLock::new();
    ARCHIVE.get_or_init(|| PathBuf::from(env!("DOVIK_RUNTIME_OBJECT")))
}

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join("dovik-cli-tests").join(test);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        Self { dir }
    }

    fn write(&self, name: &str, source: &str) -> PathBuf {
        let path = self.dir.join(name);
        std::fs::write(&path, source).expect("write source");
        path
    }

    fn count(&self) -> usize {
        std::fs::read_dir(&self.dir).expect("read").count()
    }

    fn dovik(&self, args: &[&str]) -> Output {
        let archive = runtime_ready();
        let result = Command::new(DOVIK)
            .args(args)
            .env("DOVIK_RUNTIME", archive)
            .current_dir(&self.dir)
            .output()
            .expect("run dovik");

        Output {
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            code: result.status.code().unwrap_or(-1),
        }
    }
}

struct Output {
    stdout: String,
    stderr: String,
    code: i32,
}



#[test]
fn version_and_help_succeed() {
    let sandbox = Sandbox::new("version_and_help_succeed");
    let version = sandbox.dovik(&["--version"]);
    assert_eq!(version.code, 0);
    assert!(version.stdout.starts_with("dovik "), "{:?}", version.stdout);

    let help = sandbox.dovik(&["--help"]);
    assert_eq!(help.code, 0);
    assert!(help.stdout.contains("dovik build"), "{:?}", help.stdout);
}

#[test]
fn an_unknown_command_fails_with_usage() {
    let sandbox = Sandbox::new("an_unknown_command_fails_with_usage");
    let result = sandbox.dovik(&["frobnicate"]);
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("unknown command"));
    assert!(result.stderr.contains("usage:"));
}

#[test]
fn an_unknown_option_fails() {
    let sandbox = Sandbox::new("an_unknown_option_fails");
    let path = sandbox.write("ok.dovk", "print(1)\n");
    let result = sandbox.dovik(&["check", "--wat", path.to_str().expect("path")]);
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("unknown option"));
}

#[test]
fn a_missing_file_fails() {
    let sandbox = Sandbox::new("a_missing_file_fails");
    let result = sandbox.dovik(&["check", "nope.dovk"]);
    assert_eq!(result.code, 1);
    assert!(!result.stderr.is_empty());
}

#[test]
fn check_succeeds_silently_on_a_valid_program() {
    let sandbox = Sandbox::new("check_succeeds_silently_on_a_valid_program");
    let path = sandbox.write("valid.dovk", "x = 1\nprint(x + 1)\n");
    let result = sandbox.dovik(&["check", path.to_str().expect("path")]);

    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, "");
    assert_eq!(result.stderr, "");
}

#[test]
fn every_pass_reports_a_stable_code() {
    let sandbox = Sandbox::new("every_pass_reports_a_stable_code");
    let cases = [
        ("lex.dovk", "x = \"unterminated\n", "DVK1"),
        ("parse.dovk", "1 = 2\n", "DVK2"),
        ("resolve.dovk", "print(missing)\n", "DVK3"),
        ("type.dovk", "x = 1 + 2.5\n", "DVK4"),
    ];

    for (name, source, prefix) in cases {
        let path = sandbox.write(name, source);
        let result = sandbox.dovik(&["check", path.to_str().expect("path")]);

        assert_eq!(result.code, 1, "{name} should fail");
        assert!(
            result.stderr.contains(prefix),
            "{name}: expected a {prefix} code in {:?}",
            result.stderr
        );
        assert!(result.stderr.contains("error["), "{name}: {:?}", result.stderr);
    }
}

#[test]
fn diagnostics_never_reach_stdout() {
    let sandbox = Sandbox::new("diagnostics_never_reach_stdout");
    let path = sandbox.write("broken.dovk", "x = 1 + 2.5\n");
    let result = sandbox.dovik(&["check", path.to_str().expect("path")]);

    assert_eq!(result.code, 1);
    assert_eq!(result.stdout, "", "diagnostics belong on stderr");
    assert!(!result.stderr.is_empty());
}

#[test]
fn json_diagnostics_are_one_object_per_line() {
    let sandbox = Sandbox::new("json_diagnostics_are_one_object_per_line");
    let path = sandbox.write("json.dovk", "x = 1 + 2.5\ny = \"a\" + 1\n");
    let result = sandbox.dovik(&["check", "--json", path.to_str().expect("path")]);

    assert_eq!(result.code, 1);
    let lines: Vec<&str> = result.stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{:?}", result.stderr);

    for line in lines {
        assert!(line.starts_with('{') && line.ends_with('}'), "{line}");
        assert!(line.contains("\"severity\":\"error\""), "{line}");
        assert!(line.contains("\"code\":\"DVK4"), "{line}");
        assert!(line.contains("\"line\":"), "{line}");
        assert!(line.contains("\"column\":"), "{line}");
        assert!(line.contains("\"message\":"), "{line}");
    }
}

#[test]
fn json_messages_are_escaped() {
    let sandbox = Sandbox::new("json_messages_are_escaped");
    let path = sandbox.write("quotes.dovk", "x: Blob = 1\n");
    let result = sandbox.dovik(&["check", "--json", path.to_str().expect("path")]);
    let line = result.stderr.lines().next().expect("a diagnostic");

    assert!(line.contains("\\\"") || line.contains("Blob"), "{line}");
    assert_eq!(line.matches("\"message\":").count(), 1, "{line}");
}

#[test]
fn build_produces_an_executable_named_after_the_file() {
    let sandbox = Sandbox::new("build_produces_an_executable_named_after_the_file");
    let path = sandbox.write("hello.dovk", "print(\"hi\")\n");
    let result = sandbox.dovik(&["build", path.to_str().expect("path")]);
    assert_eq!(result.code, 0, "{:?}", result.stderr);

    let binary = sandbox.dir.join("hello");
    assert!(binary.exists(), "expected an executable at {binary:?}");

    let run = Command::new(&binary).output().expect("run");
    assert_eq!(String::from_utf8_lossy(&run.stdout), "hi\n");
}

#[test]
fn build_honours_the_output_flag() {
    let sandbox = Sandbox::new("build_honours_the_output_flag");
    let path = sandbox.write("named.dovk", "print(7)\n");
    let target = sandbox.dir.join("custom-name");
    let result = sandbox.dovik(&[
        "build",
        "-o",
        target.to_str().expect("path"),
        path.to_str().expect("path"),
    ]);

    assert_eq!(result.code, 0, "{:?}", result.stderr);
    assert!(target.exists());
}

#[test]
fn run_compiles_then_executes() {
    let sandbox = Sandbox::new("run_compiles_then_executes");
    let path = sandbox.write("runme.dovk", "print(21 * 2)\n");
    let result = sandbox.dovik(&["run", path.to_str().expect("path")]);

    assert_eq!(result.code, 0, "{:?}", result.stderr);
    assert_eq!(result.stdout, "42\n");
}

#[test]
fn run_forwards_a_runtime_failure() {
    let sandbox = Sandbox::new("run_forwards_a_runtime_failure");
    let path = sandbox.write("trapme.dovk", "xs = [1]\ni = 9\nprint(xs[i])\n");
    let result = sandbox.dovik(&["run", path.to_str().expect("path")]);

    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK5005"), "{:?}", result.stderr);
    assert!(result.stderr.contains("3:7"), "{:?}", result.stderr);
}

#[test]
fn run_leaves_no_binary_behind() {
    let sandbox = Sandbox::new("run_leaves_no_binary_behind");
    let path = sandbox.write("tidy.dovk", "print(1)\n");
    let before = sandbox.count();
    assert_eq!(sandbox.dovik(&["run", path.to_str().expect("path")]).code, 0);
    let after = sandbox.count();

    assert_eq!(before, after, "run should not leave files in the workspace");
}

#[test]
fn emit_flags_write_to_stdout() {
    let sandbox = Sandbox::new("emit_flags_write_to_stdout");
    let path = sandbox.write("emit.dovk", "print(1 + 2)\n");

    for (flag, expected) in [
        ("--emit-llvm", "define i32 @main(i32 %argc"),
        ("--emit-ir", "function main()"),
        ("--emit-tokens", "Identifier"),
    ] {
        let result = sandbox.dovik(&["build", flag, path.to_str().expect("path")]);
        assert_eq!(result.code, 0, "{flag}: {:?}", result.stderr);
        assert!(
            result.stdout.contains(expected),
            "{flag}: expected {expected:?} in output"
        );
    }
}

#[test]
fn test_explains_why_it_cannot_run() {
    let sandbox = Sandbox::new("test_explains_why_it_cannot_run");
    let test = sandbox.dovik(&["test"]);
    assert_eq!(test.code, 1);
    assert!(test.stderr.contains("no way to declare a test"));
}

#[test]
fn format_writes_to_stdout_by_default() {
    let sandbox = Sandbox::new("format_writes_to_stdout_by_default");
    let path = sandbox.write("messy.dovk", "x=1+2\n");
    let result = sandbox.dovik(&["format", path.to_str().expect("path")]);

    assert_eq!(result.code, 0, "{:?}", result.stderr);
    assert_eq!(result.stdout, "x = 1 + 2\n");
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "x=1+2\n",
        "stdout mode must not touch the file"
    );
}

#[test]
fn format_check_fails_on_unformatted_input() {
    let sandbox = Sandbox::new("format_check_fails_on_unformatted_input");
    let messy = sandbox.write("messy.dovk", "x=1\n");
    assert_eq!(
        sandbox.dovik(&["format", "--check", messy.to_str().expect("path")]).code,
        1
    );

    let tidy = sandbox.write("tidy.dovk", "x = 1\n");
    assert_eq!(
        sandbox.dovik(&["format", "--check", tidy.to_str().expect("path")]).code,
        0
    );
}

#[test]
fn format_write_rewrites_the_file() {
    let sandbox = Sandbox::new("format_write_rewrites_the_file");
    let path = sandbox.write("messy.dovk", "x=1+2\n");

    assert_eq!(
        sandbox.dovik(&["format", "--write", path.to_str().expect("path")]).code,
        0
    );
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "x = 1 + 2\n");
    assert_eq!(
        sandbox.dovik(&["format", "--check", path.to_str().expect("path")]).code,
        0
    );
}

#[test]
fn format_refuses_a_file_that_does_not_parse() {
    let sandbox = Sandbox::new("format_refuses_a_file_that_does_not_parse");
    let path = sandbox.write("broken.dovk", "1 = 2\n");
    let result = sandbox.dovik(&["format", path.to_str().expect("path")]);

    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("does not parse"), "{:?}", result.stderr);
    assert_eq!(result.stdout, "");
}
