mod builtin;
mod checks;
mod collection;
mod inst;
mod predicate;
mod value;

use std::fmt::Write;

use crate::ir::{BlockId, Function, Program, Terminator};
use crate::types::Type;
use value::{escape, llvm_type};

const PREAMBLE: &str = r#"declare void @dovik_trap(i32, i32, i32)
declare void @dovik_print(i8, i64)
declare i64 @dovik_str_new(ptr, i64)
declare i64 @dovik_str_concat(i64, i64)
declare i64 @dovik_str_scalar_len(i64)
declare i64 @dovik_str_byte_len(i64)
declare i64 @dovik_str_scalar_at(i64, i64, i32, i32)
declare i64 @dovik_str_next(i64, i64, ptr)
declare i32 @dovik_str_cmp(i64, i64)
declare i64 @dovik_list_new()
declare void @dovik_list_push(i64, i8, i64)
declare i64 @dovik_list_len(i64)
declare i64 @dovik_list_get(i64, i64, i32, i32)
declare void @dovik_list_set(i64, i64, i8, i64, i32, i32)
declare i64 @dovik_dict_new()
declare void @dovik_dict_set(i64, i8, i64, i8, i64)
declare i64 @dovik_dict_get(i64, i8, i64, i32, i32)
declare i64 @dovik_dict_len(i64)
declare i8 @dovik_eq(i8, i64, i64)
declare void @dovik_list_insert(i64, i64, i8, i64, i32, i32)
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
    let mut emitter = Emitter {
        program,
        source,
        out: String::new(),
        temp: 0,
        label: String::new(),
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

        self.line("define i32 @main() {");
        self.line(&format!("  call void @dvk{}()", self.program.main));
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

        self.line(&format!(
            "define internal {result} @dvk{index}({}) {{",
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

            for item in &body.instructions {
                self.instruction(function, item);
            }

            self.terminator(function, &body.terminator);
        }

        self.line("}");
        self.line("");
    }

    fn terminator(&mut self, function: &Function, terminator: &Terminator) {
        match terminator {
            Terminator::Jump(target) => self.line(&format!("  br label %{}", block_label(*target))),
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let condition = self.native(condition);
                self.line(&format!(
                    "  br i1 {condition}, label %{}, label %{}",
                    block_label(*then_block),
                    block_label(*else_block)
                ));
            }
            Terminator::Return(Some(value)) => {
                let ty = llvm_type(&function.return_type);

                if ty == "void" {
                    return self.line("  ret void");
                }

                let operand = self.native(value);
                self.line(&format!("  ret {ty} {operand}"));
            }
            Terminator::Return(None) => match llvm_type(&function.return_type) {
                "void" => self.line("  ret void"),
                ty => self.line(&format!("  ret {ty} {}", zero_of(&function.return_type))),
            },
            Terminator::Unreachable => match llvm_type(&function.return_type) {
                "void" => self.line("  ret void"),
                ty => self.line(&format!("  ret {ty} {}", zero_of(&function.return_type))),
            },
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
