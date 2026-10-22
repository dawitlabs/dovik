use super::value::{literal, llvm_type, tag_of};
use super::predicate::{float_predicate, integer_predicate};
use super::Emitter;
use crate::ir::{ArithOp, CompareOp, Function, Inst, InstKind, Value};
use crate::types::Type;

impl Emitter<'_> {
    pub(crate) fn instruction(&mut self, function: &Function, item: &Inst) {
        let span = item.span;

        match &item.kind {
            InstKind::LoadLocal { dest, local } => {
                let ty = llvm_type(&function.locals[*local].ty);
                self.line(&format!("  %r{dest} = load {ty}, ptr %s{local}"));
            }
            InstKind::StoreLocal { local, value } => {
                let declared = function.locals[*local].ty.clone();
                let ty = llvm_type(&declared);
                let operand = self.native(value);

                if !super::ownership::is_reference(&declared) {
                    return self.line(&format!("  store {ty} {operand}, ptr %s{local}"));
                }

                let tag = tag_of(&declared);
                let previous = self.next_temp();
                self.line(&format!("  {previous} = load i64, ptr %s{local}"));
                self.line(&format!("  call void @dovik_retain(i8 {tag}, i64 {operand})"));
                self.line(&format!("  store {ty} {operand}, ptr %s{local}"));
                self.line(&format!("  call void @dovik_release(i8 {tag}, i64 {previous})"));
            }
            InstKind::LoadGlobal { dest, global } => {
                let ty = llvm_type(&self.program.globals[*global].ty);
                self.line(&format!("  %r{dest} = load {ty}, ptr @g{global}"));
            }
            InstKind::StoreGlobal { global, value } => {
                let declared = self.program.globals[*global].ty.clone();
                let ty = llvm_type(&declared);
                let operand = self.native(value);

                if !super::ownership::is_reference(&declared) {
                    return self.line(&format!("  store {ty} {operand}, ptr @g{global}"));
                }

                let tag = tag_of(&declared);
                let previous = self.next_temp();
                self.line(&format!("  {previous} = load i64, ptr @g{global}"));
                self.line(&format!("  call void @dovik_retain(i8 {tag}, i64 {operand})"));
                self.line(&format!("  store {ty} {operand}, ptr @g{global}"));
                self.line(&format!("  call void @dovik_release(i8 {tag}, i64 {previous})"));
            }
            InstKind::IntArith { dest, op, lhs, rhs } => {
                self.checked_arith(*dest, *op, lhs, rhs, span)
            }
            InstKind::FloatArith { dest, op, lhs, rhs } => {
                let name = match op {
                    ArithOp::Add => "fadd",
                    ArithOp::Sub => "fsub",
                    ArithOp::Mul => "fmul",
                };
                self.line(&format!(
                    "  %r{dest} = {name} double {}, {}",
                    literal(lhs),
                    literal(rhs)
                ));
            }
            InstKind::IntRem { dest, lhs, rhs } => self.remainder(*dest, lhs, rhs, span),
            InstKind::FloatDiv { dest, lhs, rhs } => self.divide(*dest, lhs, rhs, span),
            InstKind::IntNeg { dest, value } => {
                self.checked_arith(*dest, ArithOp::Sub, &Value::Int(0), value, span)
            }
            InstKind::FloatNeg { dest, value } => {
                self.line(&format!("  %r{dest} = fneg double {}", literal(value)));
            }
            InstKind::Not { dest, value } => {
                self.line(&format!("  %r{dest} = xor i1 {}, true", literal(value)));
            }
            InstKind::IntToFloat { dest, value } => {
                self.line(&format!(
                    "  %r{dest} = sitofp i64 {} to double",
                    literal(value)
                ));
            }
            InstKind::FloatToInt { dest, value } => self.to_int(*dest, value, span),
            InstKind::Compare { dest, op, ty, lhs, rhs } => {
                self.compare(*dest, *op, ty, lhs, rhs)
            }
            InstKind::StringConcat { dest, lhs, rhs } => {
                let left = self.string_word(lhs);
                let right = self.string_word(rhs);
                self.line(&format!(
                    "  %r{dest} = call i64 @dovik_str_concat(i64 {left}, i64 {right})"
                ));
            }
            InstKind::StringIndex { dest, string, index } => {
                let (line, column) = self.position(span);
                let target = self.string_word(string);
                self.line(&format!(
                    "  %r{dest} = call i64 @dovik_str_scalar_at(i64 {target}, i64 {}, i32 {line}, i32 {column})",
                    literal(index)
                ));
            }
            InstKind::StringNext { scalar, next_offset, string, offset } => {
                let target = self.string_word(string);
                self.line(&format!(
                    "  %r{scalar} = call i64 @dovik_str_next(i64 {target}, i64 {}, ptr %cursor)",
                    literal(offset)
                ));
                self.line(&format!("  %r{next_offset} = load i64, ptr %cursor"));
            }
            InstKind::StringByteLen { dest, string } => {
                let target = self.string_word(string);
                self.line(&format!(
                    "  %r{dest} = call i64 @dovik_str_byte_len(i64 {target})"
                ));
            }
            InstKind::ListNew { .. }
            | InstKind::ListGet { .. }
            | InstKind::ListSet { .. }
            | InstKind::DictNew { .. }
            | InstKind::DictGet { .. }
            | InstKind::DictSet { .. } => self.collection(function, &item.kind, span),
            InstKind::Len { dest, ty, value } => {
                let name = match ty {
                    Type::Str => "dovik_str_scalar_len",
                    Type::Dict(..) => "dovik_dict_len",
                    _ => "dovik_list_len",
                };
                let operand = self.native(value);
                self.line(&format!("  %r{dest} = call i64 @{name}(i64 {operand})"));
            }
            InstKind::Print { ty, value } => {
                let tag = tag_of(ty);
                let word = self.as_word(value, ty);
                self.line(&format!("  call void @dovik_print(i8 {tag}, i64 {word})"));
            }
            InstKind::Builtin { dest, which, args } => {
                self.builtin(function, *dest, *which, args, span)
            }
            InstKind::Call { dest, function: target, args } => {
                let callee = &self.program.functions[*target];
                let types: Vec<&'static str> = callee
                    .locals
                    .iter()
                    .take(args.len())
                    .map(|param| llvm_type(&param.ty))
                    .collect();
                let mut rendered: Vec<String> = Vec::with_capacity(args.len());
                for (arg, ty) in args.iter().zip(types) {
                    let operand = self.native(arg);
                    rendered.push(format!("{ty} {operand}"));
                }
                let callee = &self.program.functions[*target];
                let result = llvm_type(&callee.return_type);

                match dest {
                    Some(dest) if result != "void" => self.line(&format!(
                        "  %r{dest} = call {result} @dvk{target}({})",
                        rendered.join(", ")
                    )),
                    _ => self.line(&format!(
                        "  call {result} @dvk{target}({})",
                        rendered.join(", ")
                    )),
                }
            }
        }
    }

    pub(crate) fn type_of(&self, function: &Function, value: &Value) -> Type {
        match value {
            Value::Reg(id) => function.registers[*id].clone(),
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::Bool(_) => Type::Bool,
            Value::Str(_) => Type::Str,
        }
    }

    fn string_word(&mut self, value: &Value) -> String {
        self.as_word(value, &Type::Str)
    }

    pub(crate) fn native(&mut self, value: &Value) -> String {
        let Value::Str(id) = value else {
            return literal(value);
        };

        let temp = self.next_temp();
        self.line(&format!("  {temp} = load i64, ptr @slit.{id}"));
        temp
    }

    pub(crate) fn as_word(&mut self, value: &Value, ty: &Type) -> String {
        if matches!(value, Value::Str(_)) {
            return self.native(value);
        }

        match ty {
            Type::Float => {
                let temp = self.next_temp();
                self.line(&format!(
                    "  {temp} = bitcast double {} to i64",
                    literal(value)
                ));
                temp
            }
            Type::Bool => {
                let temp = self.next_temp();
                self.line(&format!("  {temp} = zext i1 {} to i64", literal(value)));
                temp
            }
            _ => literal(value),
        }
    }

    pub(crate) fn from_word(&mut self, dest: usize, word: &str, ty: &Type) {
        match ty {
            Type::Float => self.line(&format!("  %r{dest} = bitcast i64 {word} to double")),
            Type::Bool => self.line(&format!("  %r{dest} = trunc i64 {word} to i1")),
            _ => self.line(&format!("  %r{dest} = add i64 {word}, 0")),
        }
    }
    fn compare(&mut self, dest: usize, op: CompareOp, ty: &Type, lhs: &Value, rhs: &Value) {
        let structural = matches!(ty, Type::Str | Type::List(_) | Type::Dict(..));

        if structural && matches!(op, CompareOp::Eq | CompareOp::Ne) {
            let tag = tag_of(ty);
            let left = self.as_word(lhs, ty);
            let right = self.as_word(rhs, ty);
            let byte = self.next_temp();
            self.line(&format!(
                "  {byte} = call i8 @dovik_eq(i8 {tag}, i64 {left}, i64 {right})"
            ));
            let predicate = match op {
                CompareOp::Eq => "ne",
                _ => "eq",
            };
            self.line(&format!("  %r{dest} = icmp {predicate} i8 {byte}, 0"));
            return;
        }

        if *ty == Type::Str {
            let left = self.as_word(lhs, ty);
            let right = self.as_word(rhs, ty);
            let ordering = self.next_temp();
            self.line(&format!(
                "  {ordering} = call i32 @dovik_str_cmp(i64 {left}, i64 {right})"
            ));
            let predicate = integer_predicate(op);
            self.line(&format!(
                "  %r{dest} = icmp {predicate} i32 {ordering}, 0"
            ));
            return;
        }

        if *ty == Type::Float {
            let predicate = float_predicate(op);
            self.line(&format!(
                "  %r{dest} = fcmp {predicate} double {}, {}",
                literal(lhs),
                literal(rhs)
            ));
            return;
        }

        let width = match ty {
            Type::Bool => "i1",
            _ => "i64",
        };
        let predicate = integer_predicate(op);
        self.line(&format!(
            "  %r{dest} = icmp {predicate} {width} {}, {}",
            literal(lhs),
            literal(rhs)
        ));
    }
}
