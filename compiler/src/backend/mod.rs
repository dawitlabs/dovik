mod builtin;
mod purity;
pub(crate) mod ownership;
mod checks;
mod collection;
mod inst;
mod predicate;
mod value;

use std::fmt::Write;

use crate::ir::{BlockId, Function, Program, Terminator};
use crate::types::Type;
use ownership::is_reference;
use value::{escape, llvm_type, tag_of};

const PREAMBLE: &str = r#"declare void @dovik_trap(i32, i32, i32)
declare void @dovik_print(i8, i64)
declare i64 @dovik_str_new(ptr, i64)
declare i64 @dovik_str_concat(i64, i64)
declare i64 @dovik_str_scalar_len(i64)
declare i64 @dovik_str_byte_len(i64)
declare i64 @dovik_str_scalar_at(i64, i64, i32, i32)
declare i64 @dovik_str_next(i64, i64, ptr)
declare i32 @dovik_str_cmp(i64, i64)
declare i64 @dovik_list_new(i8)
declare i64 @dovik_list_new_sized(i8, i64)
declare void @dovik_list_push_fast(i64, i64)
declare void @dovik_list_push(i64, i64)
declare i64 @dovik_list_len(i64)
declare i64 @dovik_list_get(i64, i64, i32, i32)
declare i64 @dovik_list_get_fast(i64, i64)
declare void @dovik_list_set(i64, i64, i64, i32, i32)
declare i64 @dovik_dict_new(i8, i8)
declare void @dovik_dict_set(i64, i8, i64, i8, i64)
declare i64 @dovik_dict_get(i64, i8, i64, i32, i32)
declare i64 @dovik_dict_len(i64)
declare i8 @dovik_eq(i8, i64, i64)
declare void @dovik_retain(i8, i64)
declare void @dovik_release(i8, i64)
declare void @dovik_report_leaks()
declare i64 @dovik_test_index()
declare void @dovik_set_arguments(i32, ptr)
declare i64 @dovik_argument_count()
declare i64 @dovik_argument(i64, i32, i32)
declare i8 @dovik_file_exists(i64)
declare i64 @dovik_read_file(i64, i32, i32)
declare void @dovik_write_file(i64, i64, i32, i32)
declare void @dovik_append_file(i64, i64, i32, i32)
declare i64 @dovik_str_split(i64, i64)
declare i64 @dovik_str_join(i64, i64)
declare i64 @dovik_str_substring(i64, i64, i64, i32, i32)
declare i8 @dovik_str_contains(i64, i64)
declare i64 @dovik_str_index_of(i64, i64, i32, i32)
declare i64 @dovik_str_trim(i64)
declare i8 @dovik_str_starts_with(i64, i64)
declare i8 @dovik_str_ends_with(i64, i64)
declare i64 @dovik_str_upper(i64)
declare i64 @dovik_str_lower(i64)
declare i64 @dovik_str_replace(i64, i64, i64)
declare i64 @dovik_read_input(i32, i32)
declare void @dovik_assert(i8, i32, i32)
declare void @dovik_list_insert(i64, i64, i64, i32, i32)
declare i64 @dovik_list_remove_at(i64, i64, i32, i32)
declare i8 @dovik_dict_has_key(i64, i8, i64)
declare i8 @dovik_dict_remove_key(i64, i8, i64)
declare i64 @dovik_to_string(i8, i64)
declare i64 @dovik_parse_int(i64, i32, i32)
declare double @dovik_parse_float(i64, i32, i32)
declare i8 @dovik_can_parse_int(i64)
declare i8 @dovik_can_parse_float(i64)
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)
declare double @llvm.trunc.f64(double)
"#;

pub fn emit(program: &Program, source: &str) -> String {
    emit_with(program, source, true)
}

pub fn emit_tests(program: &Program, source: &str, tests: &[usize]) -> String {
    let mut emitter = Emitter {
        program,
        source,
        out: String::new(),
        temp: 0,
        label: String::new(),
        checks: true,
        pure: purity::classify(program),
        tests: tests.to_vec(),
    };

    emitter.module();
    emitter.out
}

pub fn emit_with(program: &Program, source: &str, checks: bool) -> String {
    let mut emitter = Emitter {
        program,
        source,
        out: String::new(),
        temp: 0,
        label: String::new(),
        checks,
        pure: purity::classify(program),
        tests: Vec::new(),
    };

    emitter.module();
    emitter.out
}

pub(crate) struct Emitter<'a> {
    pub(crate) program: &'a Program,
    pub(crate) source: &'a str,
    pub(crate) out: String,
    pub(crate) temp: usize,
    pub(crate) label: String,
    pub(crate) checks: bool,
    pub(crate) pure: Vec<bool>,
    pub(crate) tests: Vec<usize>,
}

impl Emitter<'_> {
    fn module(&mut self) {
        self.line("target triple = \"x86_64-pc-linux-gnu\"");
        self.line("");
        self.out.push_str(PREAMBLE);
        self.line("");

        for (index, text) in self.program.strings.iter().enumerate() {
            let (encoded, len) = escape(text);
            self.line(&format!(
                "@str.{index} = private unnamed_addr constant [{len} x i8] c\"{encoded}\""
            ));
            self.line(&format!("@slit.{index} = internal global i64 0"));
        }

        for (index, global) in self.program.globals.iter().enumerate() {
            let ty = llvm_type(&global.ty);
            let zero = zero_of(&global.ty);
            self.line(&format!("@g{index} = internal global {ty} {zero}"));
        }

        self.line("");

        for (index, function) in self.program.functions.iter().enumerate() {
            self.function(index, function);
        }

        self.line("define i32 @main(i32 %argc, ptr %argv) {");
        self.line("entry:");
        self.line("  call void @dovik_set_arguments(i32 %argc, ptr %argv)");

        for (index, text) in self.program.strings.iter().enumerate() {
            let len = text.len();
            let temp = self.next_temp();
            self.line(&format!(
                "  {temp} = call i64 @dovik_str_new(ptr @str.{index}, i64 {len})"
            ));
            self.line(&format!("  store i64 {temp}, ptr @slit.{index}"));
        }

        match self.tests.is_empty() {
            true => self.line(&format!("  call void @dvk{}()", self.program.main)),
            false => self.dispatch_tests(),
        }

        for (index, global) in self.program.globals.iter().enumerate() {
            if !is_reference(&global.ty) {
                continue;
            }

            let temp = self.next_temp();
            self.line(&format!("  {temp} = load i64, ptr @g{index}"));
            self.line(&format!(
                "  call void @dovik_release(i8 {}, i64 {temp})",
                tag_of(&global.ty)
            ));
        }

        for index in 0..self.program.strings.len() {
            let temp = self.next_temp();
            self.line(&format!("  {temp} = load i64, ptr @slit.{index}"));
            self.line(&format!("  call void @dovik_release(i8 3, i64 {temp})"));
        }

        self.line("  call void @dovik_report_leaks()");
        self.line("  ret i32 0");
        self.line("}");
    }

    fn function(&mut self, index: usize, function: &Function) {
        let result = llvm_type(&function.return_type);
        let params: Vec<String> = function.locals[..function.param_count]
            .iter()
            .enumerate()
            .map(|(slot, local)| format!("{} %arg{slot}", llvm_type(&local.ty)))
            .collect();

        let attributes = match self.pure.get(index).copied().unwrap_or(false) {
            true => " nounwind memory(none)",
            false => " nounwind",
        };

        self.line(&format!(
            "define internal {result} @dvk{index}({}){attributes} {{",
            params.join(", ")
        ));

        self.line("entry:");
        for (slot, local) in function.locals.iter().enumerate() {
            let ty = llvm_type(&local.ty);
            self.line(&format!("  %s{slot} = alloca {ty}"));
        }
        self.line("  %cursor = alloca i64");

        for (slot, local) in function.locals[..function.param_count].iter().enumerate() {
            let ty = llvm_type(&local.ty);
            self.line(&format!("  store {ty} %arg{slot}, ptr %s{slot}"));

            if is_reference(&local.ty) {
                self.line(&format!(
                    "  call void @dovik_retain(i8 {}, i64 %arg{slot})",
                    tag_of(&local.ty)
                ));
            }
        }

        for (slot, local) in function.locals.iter().enumerate().skip(function.param_count) {
            let ty = llvm_type(&local.ty);
            let zero = zero_of(&local.ty);
            self.line(&format!("  store {ty} {zero}, ptr %s{slot}"));
        }

        self.line("  br label %b0");

        for (block, body) in function.blocks.iter().enumerate() {
            self.label = format!("b{block}");
            self.line(&format!("{}:", self.label.clone()));

            let plan = ownership::plan(function, body);

            for (index, item) in body.instructions.iter().enumerate() {
                self.instruction(function, item);
                self.release_registers(function, &plan.after[index]);
            }

            self.terminator(function, &body.terminator, &plan.at_end);
        }

        self.line("}");
        self.line("");
    }

    fn dispatch_tests(&mut self) {
        let tests = self.tests.clone();
        let chosen = self.next_temp();
        self.line(&format!("  {chosen} = call i64 @dovik_test_index()"));

        let arms: Vec<String> = (0..tests.len())
            .map(|slot| format!("i64 {slot}, label %test{slot}"))
            .collect();

        self.line(&format!(
            "  switch i64 {chosen}, label %tests.done [ {} ]",
            arms.join(" ")
        ));

        for (slot, function) in tests.iter().enumerate() {
            self.line(&format!("test{slot}:"));
            self.line(&format!("  call void @dvk{function}()"));
            self.line("  br label %tests.done");
        }

        self.line("tests.done:");
    }

    fn release_registers(&mut self, function: &Function, registers: &[usize]) {
        for register in registers {
            let ty = function.registers[*register].clone();
            self.line(&format!(
                "  call void @dovik_release(i8 {}, i64 %r{register})",
                tag_of(&ty)
            ));
        }
    }

    fn release_frame(&mut self, function: &Function) {
        for (slot, local) in function.locals.iter().enumerate() {
            if !is_reference(&local.ty) {
                continue;
            }

            let temp = self.next_temp();
            self.line(&format!("  {temp} = load i64, ptr %s{slot}"));
            self.line(&format!(
                "  call void @dovik_release(i8 {}, i64 {temp})",
                tag_of(&local.ty)
            ));
        }
    }

    fn terminator(&mut self, function: &Function, terminator: &Terminator, held: &[usize]) {
        match terminator {
            Terminator::Jump(target) => {
                self.release_registers(function, held);
                self.line(&format!("  br label %{}", block_label(*target)))
            }
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let condition = self.native(condition);
                self.release_registers(function, held);
                self.line(&format!(
                    "  br i1 {condition}, label %{}, label %{}",
                    block_label(*then_block),
                    block_label(*else_block)
                ));
            }
            Terminator::Return(Some(value)) => {
                let ty = llvm_type(&function.return_type);

                if ty == "void" {
                    self.release_registers(function, held);
                    self.release_frame(function);
                    return self.line("  ret void");
                }

                let operand = self.native(value);

                if is_reference(&function.return_type) {
                    self.line(&format!(
                        "  call void @dovik_retain(i8 {}, i64 {operand})",
                        tag_of(&function.return_type)
                    ));
                }

                self.release_registers(function, held);
                self.release_frame(function);
                self.line(&format!("  ret {ty} {operand}"));
            }
            Terminator::Return(None) | Terminator::Unreachable => {
                self.release_registers(function, held);
                self.release_frame(function);

                match llvm_type(&function.return_type) {
                    "void" => self.line("  ret void"),
                    ty => self.line(&format!("  ret {ty} {}", zero_of(&function.return_type))),
                }
            }
        }
    }

    pub(crate) fn line(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
    }

    pub(crate) fn next_temp(&mut self) -> String {
        self.temp += 1;
        format!("%t{}", self.temp)
    }

    pub(crate) fn next_label(&mut self, prefix: &str) -> String {
        self.temp += 1;
        format!("{prefix}.{}", self.temp)
    }
}

pub(crate) fn block_label(block: BlockId) -> String {
    format!("b{block}")
}

pub(crate) fn zero_of(ty: &Type) -> &'static str {
    match ty {
        Type::Float => "0.0",
        _ => "0",
    }
}
