mod exec;

use std::collections::HashMap;

use crate::error::Trap;
use crate::ir::{self, FuncId};
use crate::runtime::Value;

const MAX_STEPS: u64 = 50_000_000;
const MAX_DEPTH: usize = 2_000;

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Finished,
    Trapped(Trap),
    LimitReached,
}

#[derive(Debug)]
pub struct Run {
    pub output: Vec<String>,
    pub outcome: Outcome,
}

impl Run {
    pub fn printed(&self) -> String {
        self.output.join("\n")
    }

    pub fn trap(&self) -> Option<&Trap> {
        match &self.outcome {
            Outcome::Trapped(trap) => Some(trap),
            _ => None,
        }
    }
}

pub fn run(program: &ir::Program) -> Run {
    let mut interpreter = Interpreter {
        program,
        globals: program
            .globals
            .iter()
            .map(|global| Value::zero_of(&global.ty))
            .collect(),
        strings: program.strings.iter().map(|text| Value::str(text.as_str())).collect(),
        output: Vec::new(),
        steps: 0,
        depth: 0,
    };

    let outcome = match interpreter.call(program.main, Vec::new()) {
        Ok(_) => Outcome::Finished,
        Err(Fault::Trap(trap)) => Outcome::Trapped(trap),
        Err(Fault::Limit) => Outcome::LimitReached,
    };

    Run {
        output: interpreter.output,
        outcome,
    }
}

pub(crate) enum Fault {
    Trap(Trap),
    Limit,
}

pub(crate) struct Interpreter<'a> {
    pub(crate) program: &'a ir::Program,
    pub(crate) globals: Vec<Value>,
    pub(crate) strings: Vec<Value>,
    pub(crate) output: Vec<String>,
    pub(crate) steps: u64,
    pub(crate) depth: usize,
}

pub(crate) struct Frame {
    pub(crate) locals: Vec<Value>,
    pub(crate) registers: Vec<Value>,
}

impl Interpreter<'_> {
    pub(crate) fn call(&mut self, id: FuncId, args: Vec<Value>) -> Result<Value, Fault> {
        if self.depth >= MAX_DEPTH {
            return Err(Fault::Limit);
        }

        let function = &self.program.functions[id];
        let mut locals: Vec<Value> = function
            .locals
            .iter()
            .map(|local| Value::zero_of(&local.ty))
            .collect();

        for (slot, value) in args.into_iter().enumerate() {
            locals[slot] = value;
        }

        let mut frame = Frame {
            locals,
            registers: vec![Value::Unit; function.registers.len()],
        };

        self.depth += 1;
        let result = self.execute(id, &mut frame);
        self.depth -= 1;

        result
    }

    pub(crate) fn tick(&mut self) -> Result<(), Fault> {
        self.steps += 1;

        match self.steps > MAX_STEPS {
            true => Err(Fault::Limit),
            false => Ok(()),
        }
    }

    pub(crate) fn string(&self, id: ir::StrId) -> Value {
        self.strings.get(id).cloned().unwrap_or(Value::str(""))
    }
}

pub(crate) fn empty_dict() -> HashMap<crate::runtime::DictKey, Value> {
    HashMap::new()
}
