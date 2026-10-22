use std::collections::HashMap;

use crate::ir::{
    ArithOp, BlockId, CompareOp, Function, GlobalId, InstKind, LocalId, RegId, Terminator, Value,
};

const WIDEN_AFTER: usize = 4;
const BUDGET: usize = 20_000;
const LIMIT: i128 = 1 << 100;
const INT_MIN: i128 = i64::MIN as i128;
const INT_MAX: i128 = i64::MAX as i128;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Range {
    Unknown,
    Known { lo: i128, hi: i128 },
}

impl Range {
    fn envelope() -> Self {
        Self::Known { lo: INT_MIN, hi: INT_MAX }
    }

    fn exact(value: i128) -> Self {
        Self::Known { lo: value, hi: value }
    }

    fn bounded(lo: i128, hi: i128) -> Self {
        if lo > hi || lo < -LIMIT || hi > LIMIT {
            return Self::Unknown;
        }

        Self::Known { lo, hi }
    }

    fn fits_in_int(self) -> bool {
        match self {
            Self::Unknown => false,
            Self::Known { lo, hi } => lo >= INT_MIN && hi <= INT_MAX,
        }
    }

    fn clamp_to_int(self) -> Self {
        match self {
            Self::Unknown => Self::Known { lo: INT_MIN, hi: INT_MAX },
            Self::Known { lo, hi } => Self::bounded(lo.max(INT_MIN), hi.min(INT_MAX)),
        }
    }

    fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Known { lo: a, hi: b }, Self::Known { lo: c, hi: d }) => {
                Self::bounded(a.min(c), b.max(d))
            }
            _ => Self::Unknown,
        }
    }

    fn widen(self, previous: Self) -> Self {
        let (Self::Known { lo, hi }, Self::Known { lo: plo, hi: phi }) = (self, previous) else {
            return Self::Unknown;
        };

        let lo = if lo < plo { INT_MIN } else { lo };
        let hi = if hi > phi { INT_MAX } else { hi };
        Self::bounded(lo, hi)
    }

    fn arith(self, op: ArithOp, other: Self) -> Self {
        let (Self::Known { lo: a, hi: b }, Self::Known { lo: c, hi: d }) = (self, other) else {
            return Self::Unknown;
        };

        match op {
            ArithOp::Add => Self::bounded(a + c, b + d),
            ArithOp::Sub => Self::bounded(a - d, b - c),
            ArithOp::Mul => {
                let corners = [a * c, a * d, b * c, b * d];
                let lo = corners.iter().copied().min().unwrap_or(0);
                let hi = corners.iter().copied().max().unwrap_or(0);
                Self::bounded(lo, hi)
            }
        }
    }

    fn negate(self) -> Self {
        match self {
            Self::Unknown => Self::Unknown,
            Self::Known { lo, hi } => Self::bounded(-hi, -lo),
        }
    }

    fn remainder(self, divisor: Self) -> Self {
        let Self::Known { lo: dlo, hi: dhi } = divisor else {
            return Self::Unknown;
        };

        if dlo <= 0 && dhi >= 0 {
            return Self::Unknown;
        }

        let magnitude = dlo.abs().max(dhi.abs()) - 1;
        let non_negative = matches!(self, Self::Known { lo, .. } if lo >= 0);

        match non_negative {
            true => Self::bounded(0, magnitude),
            false => Self::bounded(-magnitude, magnitude),
        }
    }

    fn refine(self, op: CompareOp, bound: Self) -> Self {
        let (Self::Known { lo, hi }, Self::Known { lo: blo, hi: bhi }) = (self, bound) else {
            return self;
        };

        match op {
            CompareOp::Lt => Self::bounded(lo, hi.min(bhi - 1)),
            CompareOp::Le => Self::bounded(lo, hi.min(bhi)),
            CompareOp::Gt => Self::bounded(lo.max(blo + 1), hi),
            CompareOp::Ge => Self::bounded(lo.max(blo), hi),
            CompareOp::Eq => Self::bounded(lo.max(blo), hi.min(bhi)),
            CompareOp::Ne => self,
        }
    }
}

fn invert(op: CompareOp) -> CompareOp {
    match op {
        CompareOp::Lt => CompareOp::Ge,
        CompareOp::Le => CompareOp::Gt,
        CompareOp::Gt => CompareOp::Le,
        CompareOp::Ge => CompareOp::Lt,
        CompareOp::Eq => CompareOp::Ne,
        CompareOp::Ne => CompareOp::Eq,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Place {
    Local(LocalId),
    Global(GlobalId),
}

#[derive(Clone, PartialEq, Eq, Default)]
struct State {
    slots: HashMap<LocalId, Range>,
    globals: HashMap<GlobalId, Range>,
    regs: HashMap<RegId, Range>,
}

impl State {
    fn slot(&self, slot: LocalId) -> Range {
        self.slots.get(&slot).copied().unwrap_or(Range::envelope())
    }

    fn global(&self, global: GlobalId) -> Range {
        self.globals.get(&global).copied().unwrap_or(Range::envelope())
    }

    fn place(&self, place: Place) -> Range {
        match place {
            Place::Local(slot) => self.slot(slot),
            Place::Global(global) => self.global(global),
        }
    }

    fn set_place(&mut self, place: Place, range: Range) {
        match place {
            Place::Local(slot) => self.slots.insert(slot, range),
            Place::Global(global) => self.globals.insert(global, range),
        };
    }

    fn value(&self, value: &Value) -> Range {
        match value {
            Value::Int(inner) => Range::exact(*inner as i128),
            Value::Bool(inner) => Range::exact(i128::from(*inner)),
            Value::Reg(register) => self.regs.get(register).copied().unwrap_or(Range::envelope()),
            _ => Range::Unknown,
        }
    }

    fn prune(mut self) -> Self {
        let top = Range::envelope();
        self.slots.retain(|_, range| *range != top && *range != Range::Unknown);
        self.globals.retain(|_, range| *range != top && *range != Range::Unknown);
        self.regs.retain(|_, range| *range != top && *range != Range::Unknown);
        self
    }

    fn join(&self, other: &Self) -> Self {
        let mut joined = State::default();

        for key in self.slots.keys().chain(other.slots.keys()) {
            joined.slots.insert(*key, self.slot(*key).join(other.slot(*key)));
        }

        for key in self.globals.keys().chain(other.globals.keys()) {
            joined.globals.insert(*key, self.global(*key).join(other.global(*key)));
        }

        for key in self.regs.keys().chain(other.regs.keys()) {
            let a = self.regs.get(key).copied().unwrap_or(Range::envelope());
            let b = other.regs.get(key).copied().unwrap_or(Range::envelope());
            joined.regs.insert(*key, a.join(b));
        }

        joined.prune()
    }

    fn widen(&self, previous: &Self) -> Self {
        let mut widened = State::default();

        for (key, value) in &self.slots {
            widened.slots.insert(*key, value.widen(previous.slot(*key)));
        }

        for (key, value) in &self.globals {
            widened.globals.insert(*key, value.widen(previous.global(*key)));
        }

        for (key, value) in &self.regs {
            let before = previous.regs.get(key).copied().unwrap_or(Range::envelope());
            widened.regs.insert(*key, value.widen(before));
        }

        widened.prune()
    }
}

struct Facts {
    loaded_from: HashMap<RegId, Place>,
    comparisons: HashMap<RegId, (CompareOp, Value, Value)>,
    predecessors: Vec<Vec<BlockId>>,
    headers: Vec<bool>,
}

fn collect(function: &Function) -> Facts {
    let mut loaded_from = HashMap::new();
    let mut comparisons = HashMap::new();
    let mut predecessors = vec![Vec::new(); function.blocks.len()];

    for (index, block) in function.blocks.iter().enumerate() {
        for inst in &block.instructions {
            match &inst.kind {
                InstKind::LoadLocal { dest, local } => {
                    loaded_from.insert(*dest, Place::Local(*local));
                }
                InstKind::LoadGlobal { dest, global } => {
                    loaded_from.insert(*dest, Place::Global(*global));
                }
                InstKind::Compare { dest, op, lhs, rhs, .. } => {
                    comparisons.insert(*dest, (*op, lhs.clone(), rhs.clone()));
                }
                _ => {}
            }
        }

        match &block.terminator {
            Terminator::Jump(target) => predecessors[*target].push(index),
            Terminator::Branch { then_block, else_block, .. } => {
                predecessors[*then_block].push(index);
                predecessors[*else_block].push(index);
            }
            _ => {}
        }
    }

    let headers = (0..function.blocks.len())
        .map(|block| predecessors[block].iter().any(|pred| *pred >= block))
        .collect();

    Facts { loaded_from, comparisons, predecessors, headers }
}

fn transfer(function: &Function, block: BlockId, state: &State) -> State {
    let mut current = state.clone();

    for inst in &function.blocks[block].instructions {
        current = step(&inst.kind, current);
    }

    current.prune()
}

fn narrow(facts: &Facts, function: &Function, from: BlockId, to: BlockId, state: &State) -> State {
    let Terminator::Branch { condition, then_block, else_block } =
        &function.blocks[from].terminator
    else {
        return state.clone();
    };

    let Value::Reg(register) = condition else {
        return state.clone();
    };

    let Some((op, lhs, rhs)) = facts.comparisons.get(register) else {
        return state.clone();
    };

    let taken = if to == *then_block {
        *op
    } else if to == *else_block {
        invert(*op)
    } else {
        return state.clone();
    };

    let mut refined = state.clone();
    let bound = state.value(rhs);
    let finish = |refined: State| refined.prune();

    if let Value::Reg(left) = lhs {
        let narrowed = state.value(lhs).refine(taken, bound);
        refined.regs.insert(*left, narrowed);

        if let Some(place) = facts.loaded_from.get(left) {
            let refined_place = state.place(*place).refine(taken, bound);
            refined.set_place(*place, refined_place);
        }
    }

    finish(refined)
}

pub(crate) fn elide_checks(function: &mut Function) -> usize {
    if function.blocks.is_empty() {
        return 0;
    }

    let facts = collect(function);
    let mut entry: Vec<State> = vec![State::default(); function.blocks.len()];
    let mut visits = vec![0usize; function.blocks.len()];
    let mut processed = vec![false; function.blocks.len()];
    let mut queue = vec![0usize];

    let mut budget = BUDGET;

    while let Some(block) = queue.pop() {
        budget = match budget.checked_sub(1) {
            Some(remaining) => remaining,
            None => return 0,
        };

        let incoming = match facts.predecessors[block].is_empty() {
            true => State::default(),
            false => {
                let mut merged: Option<State> = None;

                for predecessor in &facts.predecessors[block] {
                    if !processed[*predecessor] {
                        continue;
                    }

                    let out = transfer(function, *predecessor, &entry[*predecessor]);
                    let edge = narrow(&facts, function, *predecessor, block, &out);
                    merged = Some(match merged {
                        Some(existing) => existing.join(&edge),
                        None => edge,
                    });
                }

                merged.unwrap_or_default()
            }
        };

        visits[block] += 1;

        let merged = match processed[block] {
            true => entry[block].join(&incoming),
            false => incoming,
        };

        let merged = match facts.headers[block] && visits[block] > WIDEN_AFTER {
            true => merged.widen(&entry[block]),
            false => merged,
        };

        if processed[block] && merged == entry[block] {
            continue;
        }

        entry[block] = merged;
        processed[block] = true;

        match &function.blocks[block].terminator {
            Terminator::Jump(target) => queue.push(*target),
            Terminator::Branch { then_block, else_block, .. } => {
                queue.push(*then_block);
                queue.push(*else_block);
            }
            _ => {}
        }
    }

    let mut elided = 0;

    for block in 0..function.blocks.len() {
        if !processed[block] {
            continue;
        }

        let mut state = entry[block].clone();

        for index in 0..function.blocks[block].instructions.len() {
            let provable = provably_in_range(&function.blocks[block].instructions[index].kind, &state);

            if provable {
                match &mut function.blocks[block].instructions[index].kind {
                    InstKind::IntArith { checked, .. } | InstKind::IntNeg { checked, .. } => {
                        if *checked {
                            *checked = false;
                            elided += 1;
                        }
                    }
                    _ => {}
                }
            }

            state = step(&function.blocks[block].instructions[index].kind, state);
        }
    }

    elided
}

fn step(kind: &InstKind, state: State) -> State {
    let mut state = state;

    match kind {
        InstKind::LoadLocal { dest, local } => {
            let range = state.slot(*local);
            state.regs.insert(*dest, range);
        }
        InstKind::StoreLocal { local, value } => {
            let range = state.value(value);
            state.slots.insert(*local, range);
        }
        InstKind::LoadGlobal { dest, global } => {
            let range = state.global(*global);
            state.regs.insert(*dest, range);
        }
        InstKind::StoreGlobal { global, value } => {
            let range = state.value(value);
            state.globals.insert(*global, range);
        }
        InstKind::IntArith { dest, op, lhs, rhs, .. } => {
            let range = state.value(lhs).arith(*op, state.value(rhs));
            state.regs.insert(*dest, range.clamp_to_int());
        }
        InstKind::IntNeg { dest, value, .. } => {
            let range = state.value(value).negate();
            state.regs.insert(*dest, range.clamp_to_int());
        }
        InstKind::IntRem { dest, lhs, rhs } => {
            let range = state.value(lhs).remainder(state.value(rhs));
            state.regs.insert(*dest, range);
        }
        InstKind::Len { dest, .. } | InstKind::StringByteLen { dest, .. } => {
            state.regs.insert(*dest, Range::bounded(0, INT_MAX));
        }
        InstKind::Compare { dest, .. } | InstKind::Not { dest, .. } => {
            state.regs.insert(*dest, Range::bounded(0, 1));
        }
        InstKind::FloatToInt { dest, .. } => {
            state.regs.insert(*dest, Range::bounded(INT_MIN, INT_MAX));
        }
        other => {
            for dest in super::escape::destinations(other) {
                state.regs.insert(dest, Range::Unknown);
            }
        }
    }

    state
}

fn provably_in_range(kind: &InstKind, state: &State) -> bool {
    match kind {
        InstKind::IntArith { op, lhs, rhs, .. } => {
            state.value(lhs).arith(*op, state.value(rhs)).fits_in_int()
        }
        InstKind::IntNeg { value, .. } => state.value(value).negate().fits_in_int(),
        _ => false,
    }
}
