use super::{Fault, Frame, Interpreter};
use crate::error::Trap;
use crate::ir::{ArithOp, FuncId, InstKind, RuntimeError, Terminator, Value as IrValue};
use crate::runtime::{self, Value};
use crate::token::Span;
use crate::types::Type;

impl Interpreter<'_> {
    pub(crate) fn execute(&mut self, id: FuncId, frame: &mut Frame) -> Result<Value, Fault> {
        let mut block = 0;

        loop {
            self.tick()?;
            let count = self.program.functions[id].blocks[block].instructions.len();

            for index in 0..count {
                self.tick()?;
                self.step(id, block, index, frame)?;
            }

            match &self.program.functions[id].blocks[block].terminator {
                Terminator::Jump(target) => block = *target,
                Terminator::Branch {
                    condition,
                    then_block,
                    else_block,
                } => {
                    let taken = self.value(condition, frame).as_bool();
                    block = match taken {
                        true => *then_block,
                        false => *else_block,
                    };
                }
                Terminator::Return(value) => {
                    return Ok(match value {
                        Some(value) => self.value(value, frame),
                        None => Value::Unit,
                    });
                }
                Terminator::Unreachable => return Ok(Value::Unit),
            }
        }
    }

    fn step(
        &mut self,
        id: FuncId,
        block: usize,
        index: usize,
        frame: &mut Frame,
    ) -> Result<(), Fault> {
        let span = self.program.functions[id].blocks[block].instructions[index].span;

        match &self.program.functions[id].blocks[block].instructions[index].kind {
            InstKind::LoadLocal { dest, local } => {
                let value = frame.locals[*local].clone();
                frame.registers[*dest] = value;
            }
            InstKind::StoreLocal { local, value } => {
                let value = self.value(value, frame);
                frame.locals[*local] = value;
            }
            InstKind::LoadGlobal { dest, global } => {
                frame.registers[*dest] = self.globals[*global].clone();
            }
            InstKind::StoreGlobal { global, value } => {
                let value = self.value(value, frame);
                self.globals[*global] = value;
            }
            InstKind::IntArith { dest, op, lhs, rhs }
            | InstKind::FloatArith { dest, op, lhs, rhs } => {
                let lhs = self.value(lhs, frame);
                let rhs = self.value(rhs, frame);
                let result = match op {
                    ArithOp::Add => runtime::add(&lhs, &rhs),
                    ArithOp::Sub => runtime::subtract(&lhs, &rhs),
                    ArithOp::Mul => runtime::multiply(&lhs, &rhs),
                };
                frame.registers[*dest] = trap(result, span)?;
            }
            InstKind::IntRem { dest, lhs, rhs } => {
                let lhs = self.value(lhs, frame);
                let rhs = self.value(rhs, frame);
                frame.registers[*dest] = trap(runtime::remainder(&lhs, &rhs), span)?;
            }
            InstKind::FloatDiv { dest, lhs, rhs } => {
                let lhs = self.value(lhs, frame);
                let rhs = self.value(rhs, frame);
                frame.registers[*dest] = trap(runtime::divide(&lhs, &rhs), span)?;
            }
            InstKind::IntNeg { dest, value } | InstKind::FloatNeg { dest, value } => {
                let value = self.value(value, frame);
                frame.registers[*dest] = trap(runtime::negate(&value), span)?;
            }
            InstKind::Not { dest, value } => {
                let value = self.value(value, frame);
                frame.registers[*dest] = Value::Bool(!value.as_bool());
            }
            InstKind::IntToFloat { dest, value } => {
                let value = self.value(value, frame);
                frame.registers[*dest] = Value::Float(value.as_float());
            }
            InstKind::FloatToInt { dest, value } => {
                let value = self.value(value, frame);
                frame.registers[*dest] = trap(runtime::to_int(&value), span)?;
            }
            InstKind::Compare { dest, op, lhs, rhs, .. } => {
                let lhs = self.value(lhs, frame);
                let rhs = self.value(rhs, frame);
                frame.registers[*dest] = runtime::compare(*op, &lhs, &rhs);
            }
            InstKind::StringConcat { dest, lhs, rhs } => {
                let lhs = self.value(lhs, frame);
                let rhs = self.value(rhs, frame);
                frame.registers[*dest] = runtime::concat(&lhs, &rhs);
            }
            InstKind::StringIndex { dest, string, index } => {
                let string = self.value(string, frame);
                let index = self.value(index, frame).as_int();
                frame.registers[*dest] = trap(runtime::scalar_at(string.as_str(), index), span)?;
            }
            InstKind::StringNext { scalar, next_offset, string, offset } => {
                let string = self.value(string, frame);
                let offset = self.value(offset, frame).as_int();
                let (value, next) = runtime::next_scalar(string.as_str(), offset);
                frame.registers[*scalar] = value;
                frame.registers[*next_offset] = Value::Int(next);
            }
            InstKind::StringByteLen { dest, string } => {
                let string = self.value(string, frame);
                frame.registers[*dest] = Value::Int(runtime::byte_len(string.as_str()));
            }
            InstKind::ListNew { dest, items, .. } => {
                let items: Vec<Value> = items.iter().map(|item| self.value(item, frame)).collect();
                frame.registers[*dest] = Value::list(items);
            }
            InstKind::ListGet { dest, list, index, .. } => {
                let list = self.value(list, frame);
                let index = self.value(index, frame).as_int();
                let Value::List(items) = &list else {
                    return Err(fault(RuntimeError::IndexOutOfBounds, span));
                };
                frame.registers[*dest] = trap(runtime::list_get(items, index), span)?;
            }
            InstKind::ListSet { list, index, value } => {
                let list = self.value(list, frame);
                let index = self.value(index, frame).as_int();
                let value = self.value(value, frame);
                let Value::List(items) = &list else {
                    return Err(fault(RuntimeError::IndexOutOfBounds, span));
                };
                trap(runtime::list_set(items, index, value), span)?;
            }
            InstKind::DictNew { dest, entries, .. } => {
                let mut map = super::empty_dict();
                for (key, value) in entries {
                    let key = self.value(key, frame);
                    let value = self.value(value, frame);
                    map.insert(key.as_key(), value);
                }
                frame.registers[*dest] = Value::dict(map);
            }
            InstKind::DictGet { dest, dict, key } => {
                let dict = self.value(dict, frame);
                let key = self.value(key, frame);
                let Value::Dict(entries) = &dict else {
                    return Err(fault(RuntimeError::MissingKey, span));
                };
                frame.registers[*dest] = trap(runtime::dict_get(entries, &key), span)?;
            }
            InstKind::DictSet { dict, key, value } => {
                let dict = self.value(dict, frame);
                let key = self.value(key, frame);
                let value = self.value(value, frame);
                if let Value::Dict(entries) = &dict {
                    runtime::dict_set(entries, &key, value);
                }
            }
            InstKind::Len { dest, value, .. } => {
                let value = self.value(value, frame);
                frame.registers[*dest] = Value::Int(runtime::len(&value));
            }
            InstKind::Print { value, ty } => {
                let value = self.value(value, frame);
                self.output.push(rendered(&value, ty));
            }
            InstKind::Builtin { dest, which, args } => {
                let which = *which;
                let dest = *dest;
                let args = args.clone();
                let result = self.run_builtin(which, &args, frame, span)?;

                if let Some(dest) = dest {
                    frame.registers[dest] = result;
                }
            }
            InstKind::Call { dest, function, args } => {
                let args: Vec<Value> = args.iter().map(|arg| self.value(arg, frame)).collect();
                let result = self.call(*function, args)?;
                if let Some(dest) = dest {
                    frame.registers[*dest] = result;
                }
            }
        }

        Ok(())
    }

    pub(crate) fn value(&self, value: &IrValue, frame: &Frame) -> Value {
        match value {
            IrValue::Reg(id) => frame.registers[*id].clone(),
            IrValue::Int(inner) => Value::Int(*inner),
            IrValue::Float(inner) => Value::Float(*inner),
            IrValue::Bool(inner) => Value::Bool(*inner),
            IrValue::Str(id) => self.string(*id),
        }
    }
}

fn rendered(value: &Value, ty: &Type) -> String {
    match ty {
        Type::Float => runtime::float(value.as_float()),
        _ => value.to_string(),
    }
}

fn trap<T>(result: Result<T, RuntimeError>, span: Span) -> Result<T, Fault> {
    result.map_err(|error| fault(error, span))
}

fn fault(error: RuntimeError, span: Span) -> Fault {
    Fault::Trap(Trap { error, span })
}
