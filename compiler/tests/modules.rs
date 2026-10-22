use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const DOVIK: &str = env!("CARGO_BIN_EXE_dovik");

fn runtime_ready() -> &'static PathBuf {
    static ARCHIVE: OnceLock<PathBuf> = OnceLock::new();
    ARCHIVE.get_or_init(|| PathBuf::from(env!("DOVIK_RUNTIME_OBJECT")))
}

struct Project {
    dir: PathBuf,
}

struct Output {
    stdout: String,
    stderr: String,
    code: i32,
}

impl Project {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join("dovik-module-tests").join(test);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        Self { dir }
    }

    fn write(&self, name: &str, source: &str) {
        let path = self.dir.join(name);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("nested dir");
        }

        std::fs::write(&path, source).expect("write source");
    }

    fn dovik(&self, args: &[&str]) -> Output {
        let result = Command::new(DOVIK)
            .args(args)
            .env("DOVIK_RUNTIME", runtime_ready())
            .env("DOVIK_CHECK_LEAKS", "1")
            .current_dir(&self.dir)
            .output()
            .expect("run dovik");

        Output {
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            code: result.status.code().unwrap_or(-1),
        }
    }

    fn run(&self, entry: &str) -> Output {
        self.dovik(&["run", entry])
    }

    fn check(&self, entry: &str) -> Output {
        self.dovik(&["check", entry])
    }
}

#[test]
fn an_import_brings_in_functions_and_variables() {
    let project = Project::new("basic");
    project.write(
        "lib/text.dovk",
        "tab = \"\\t\"\n\nfunction flatten(text: String) -> String:\n    return join(split(text, tab), \" \")\n",
    );
    project.write(
        "main.dovk",
        "import \"lib/text.dovk\" (flatten, tab)\n\nprint(flatten(\"a\\tb\"))\nprint(len(tab))\n",
    );

    let result = project.run("main.dovk");
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.stdout, "a b\n1\n");
    assert!(
        result.stderr.contains("0 live object"),
        "leaked: {}",
        result.stderr
    );
}

#[test]
fn a_dependency_is_initialized_before_the_module_that_imports_it() {
    let project = Project::new("order");
    project.write("dep.dovk", "print(\"dependency\")\nvalue = 7\n");
    project.write(
        "main.dovk",
        "import \"dep.dovk\" (value)\n\nprint(\"entry\")\nprint(value)\n",
    );

    let result = project.run("main.dovk");
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.stdout, "dependency\nentry\n7\n");
}

#[test]
fn a_module_reached_twice_is_initialized_once() {
    let project = Project::new("diamond");
    project.write("base.dovk", "print(\"base\")\nname = \"base\"\n");
    project.write(
        "left.dovk",
        "import \"base.dovk\" (name)\n\nfunction left() -> String:\n    return name + \"-left\"\n",
    );
    project.write(
        "right.dovk",
        "import \"base.dovk\" (name)\n\nfunction right() -> String:\n    return name + \"-right\"\n",
    );
    project.write(
        "main.dovk",
        "import \"left.dovk\" (left)\nimport \"right.dovk\" (right)\n\nprint(left())\nprint(right())\n",
    );

    let result = project.run("main.dovk");
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.stdout, "base\nbase-left\nbase-right\n");
}

#[test]
fn two_paths_to_one_file_are_one_module() {
    let project = Project::new("same_file");
    project.write("once.dovk", "print(\"once\")\nvalue = 1\n");
    project.write(
        "side.dovk",
        "import \"./once.dovk\" (value)\n\nfunction side() -> Int:\n    return value\n",
    );
    project.write(
        "main.dovk",
        "import \"once.dovk\" (value)\nimport \"side.dovk\" (side)\n\nprint(value + side())\n",
    );

    let result = project.run("main.dovk");
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.stdout, "once\n2\n");
}

#[test]
fn as_renames_and_the_original_name_stays_hidden() {
    let project = Project::new("alias");
    project.write("lib.dovk", "function value() -> Int:\n    return 3\n");
    project.write(
        "main.dovk",
        "import \"lib.dovk\" (value as amount)\n\nprint(amount())\n",
    );
    project.write(
        "wrong.dovk",
        "import \"lib.dovk\" (value as amount)\n\nprint(value())\n",
    );

    assert_eq!(project.run("main.dovk").stdout, "3\n");

    let rejected = project.check("wrong.dovk");
    assert_eq!(rejected.code, 1);
    assert!(
        rejected.stderr.contains("DVK3001"),
        "{}",
        rejected.stderr
    );
}

#[test]
fn private_names_may_repeat_across_modules() {
    let project = Project::new("repeat");
    project.write(
        "one.dovk",
        "function helper() -> Int:\n    return 1\n\nfunction one() -> Int:\n    return helper()\n",
    );
    project.write(
        "two.dovk",
        "function helper() -> Int:\n    return 2\n\nfunction two() -> Int:\n    return helper()\n",
    );
    project.write(
        "main.dovk",
        "import \"one.dovk\" (one)\nimport \"two.dovk\" (two)\n\nprint(one())\nprint(two())\n",
    );

    let result = project.run("main.dovk");
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.stdout, "1\n2\n");
}

#[test]
fn imports_do_not_re_export() {
    let project = Project::new("no_reexport");
    project.write("base.dovk", "function base() -> Int:\n    return 1\n");
    project.write(
        "middle.dovk",
        "import \"base.dovk\" (base)\n\nfunction middle() -> Int:\n    return base()\n",
    );
    project.write(
        "main.dovk",
        "import \"middle.dovk\" (base)\n\nprint(base())\n",
    );

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK3013"), "{}", result.stderr);
    assert!(
        result.stderr.contains("does not declare `base`"),
        "{}",
        result.stderr
    );
}

#[test]
fn a_cycle_is_rejected_and_named() {
    let project = Project::new("cycle");
    project.write(
        "a.dovk",
        "import \"b.dovk\" (from_b)\n\nfunction from_a() -> Int:\n    return 1\n",
    );
    project.write(
        "b.dovk",
        "import \"a.dovk\" (from_a)\n\nfunction from_b() -> Int:\n    return 2\n",
    );
    project.write("main.dovk", "import \"a.dovk\" (from_a)\n\nprint(from_a())\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK6002"), "{}", result.stderr);
    assert!(result.stderr.contains("a.dovk"), "{}", result.stderr);
    assert!(result.stderr.contains("b.dovk"), "{}", result.stderr);
}

#[test]
fn a_module_importing_itself_is_a_cycle() {
    let project = Project::new("self_import");
    project.write("main.dovk", "import \"main.dovk\" (value)\n\nvalue = 1\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK6002"), "{}", result.stderr);
}

#[test]
fn a_missing_module_is_reported_at_its_import() {
    let project = Project::new("missing");
    project.write("main.dovk", "import \"ghost.dovk\" (anything)\n\nprint(1)\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK6001"), "{}", result.stderr);
    assert!(result.stderr.contains("main.dovk:1:8"), "{}", result.stderr);
}

#[test]
fn an_absolute_path_is_rejected() {
    let project = Project::new("absolute");
    project.write("main.dovk", "import \"/etc/passwd\" (anything)\n\nprint(1)\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK2009"), "{}", result.stderr);
}

#[test]
fn an_import_after_a_statement_is_rejected() {
    let project = Project::new("not_first");
    project.write("lib.dovk", "value = 1\n");
    project.write("main.dovk", "print(1)\nimport \"lib.dovk\" (value)\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK2007"), "{}", result.stderr);
}

#[test]
fn an_empty_import_list_is_rejected() {
    let project = Project::new("empty_list");
    project.write("lib.dovk", "value = 1\n");
    project.write("main.dovk", "import \"lib.dovk\" ()\n\nprint(1)\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK2008"), "{}", result.stderr);
}

#[test]
fn importing_an_undeclared_name_is_rejected() {
    let project = Project::new("no_export");
    project.write("lib.dovk", "value = 1\n");
    project.write("main.dovk", "import \"lib.dovk\" (missing)\n\nprint(1)\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK3013"), "{}", result.stderr);
}

#[test]
fn an_imported_name_cannot_be_assigned_at_module_scope() {
    let project = Project::new("assign_import");
    project.write("lib.dovk", "value = 1\n");
    project.write(
        "main.dovk",
        "import \"lib.dovk\" (value)\n\nvalue = 2\nprint(value)\n",
    );

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK3009"), "{}", result.stderr);
    assert!(
        result.stderr.contains("an imported name"),
        "{}",
        result.stderr
    );
}

#[test]
fn an_import_cannot_collide_with_a_function_of_the_same_name() {
    let project = Project::new("collide_local");
    project.write("lib.dovk", "function value() -> Int:\n    return 1\n");
    project.write(
        "main.dovk",
        "import \"lib.dovk\" (value)\n\nfunction value() -> Int:\n    return 2\n\nprint(value())\n",
    );

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK3002"), "{}", result.stderr);
}

#[test]
fn an_import_cannot_collide_with_a_builtin() {
    let project = Project::new("collide_builtin");
    project.write("lib.dovk", "function mine() -> Int:\n    return 1\n");
    project.write("main.dovk", "import \"lib.dovk\" (mine as len)\n\nprint(1)\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK3005"), "{}", result.stderr);
}

#[test]
fn two_imports_of_one_name_collide() {
    let project = Project::new("collide_imports");
    project.write("one.dovk", "function same() -> Int:\n    return 1\n");
    project.write("two.dovk", "function same() -> Int:\n    return 2\n");
    project.write(
        "main.dovk",
        "import \"one.dovk\" (same)\nimport \"two.dovk\" (same)\n\nprint(same())\n",
    );

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK3002"), "{}", result.stderr);
}

#[test]
fn a_diagnostic_names_the_file_it_came_from() {
    let project = Project::new("which_file");
    project.write("lib.dovk", "function broken() -> Int:\n    return \"text\"\n");
    project.write("main.dovk", "import \"lib.dovk\" (broken)\n\nprint(broken())\n");

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("lib.dovk:2:"), "{}", result.stderr);
}

#[test]
fn an_imported_global_cannot_be_assigned_from_a_function() {
    let project = Project::new("imported_global");
    project.write("lib.dovk", "counter = 0\n");
    project.write(
        "main.dovk",
        "import \"lib.dovk\" (counter)\n\nfunction bump():\n    counter = counter + 1\n\nbump()\n",
    );

    let result = project.check("main.dovk");
    assert_eq!(result.code, 1);
    assert!(result.stderr.contains("DVK3009"), "{}", result.stderr);
}

#[test]
fn dovik_test_collects_only_the_entry_file() {
    let project = Project::new("tests_entry_only");
    project.write(
        "lib.dovk",
        "function double(n: Int) -> Int:\n    return n * 2\n\nfunction test_not_collected():\n    assert(false)\n",
    );
    project.write(
        "suite.dovk",
        "import \"lib.dovk\" (double)\n\nfunction test_doubling():\n    assert(double(2) == 4)\n",
    );

    let result = project.dovik(&["test", "suite.dovk"]);
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    assert!(result.stdout.contains("ok   test_doubling"), "{}", result.stdout);
    assert!(
        !result.stdout.contains("test_not_collected"),
        "{}",
        result.stdout
    );
    assert!(result.stdout.contains("1 passed, 0 failed"), "{}", result.stdout);
}

#[test]
fn a_deep_chain_compiles_in_dependency_order() {
    let project = Project::new("chain");
    project.write("one.dovk", "print(\"one\")\na = 1\n");
    project.write("two.dovk", "import \"one.dovk\" (a)\nprint(\"two\")\nb = a + 1\n");
    project.write("three.dovk", "import \"two.dovk\" (b)\nprint(\"three\")\nc = b + 1\n");
    project.write("main.dovk", "import \"three.dovk\" (c)\n\nprint(c)\n");

    let result = project.run("main.dovk");
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.stdout, "one\ntwo\nthree\n3\n");
}

#[test]
fn a_parent_directory_import_works() {
    let project = Project::new("parent");
    project.write("shared.dovk", "value = 5\n");
    project.write(
        "nested/main.dovk",
        "import \"../shared.dovk\" (value)\n\nprint(value)\n",
    );

    let result = project.run("nested/main.dovk");
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.stdout, "5\n");
}
