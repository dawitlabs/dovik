use super::value::literal;
use super::Emitter;
use crate::error::line_col;
use crate::ir::{ArithOp, Value};
use crate::token::Span;

impl Emitter<'_> {
    pub(crate) fn position(&self, span: Span) -> (usize, usize) {
        line_col(self.source, span.start)
    }

    pub(crate) fn trap_if(&mut self, condition: &str, code: u8, span: Span) {
        if !self.checks {
            return;
        }

        let (line, column) = self.position(span);
        let trap = self.next_label("trap");
        let cont = self.next_label("cont");

        self.line(&format!(
            "  br i1 {condition}, label %{trap}, label %{cont}"
        ));
        self.line(&format!("{trap}:"));
        self.line(&format!(
            "  call void @dovik_trap(i32 {code}, i32 {line}, i32 {column})"
        ));
        self.line("  unreachable");
        self.line(&format!("{cont}:"));
        self.label = cont;
    }

    pub(crate) fn checked_arith(&mut self, dest: usize, op: ArithOp, lhs: &Value, rhs: &Value, span: Span) {
        if !self.checks {
            let plain = match op {
                ArithOp::Add => "add",
                ArithOp::Sub => "sub",
                ArithOp::Mul => "mul",
            };
            return self.line(&format!(
                "  %r{dest} = {plain} i64 {}, {}",
                literal(lhs),
                literal(rhs)
            ));
        }

        let intrinsic = match op {
            ArithOp::Add => "sadd",
            ArithOp::Sub => "ssub",
            ArithOp::Mul => "smul",
        };

        let pair = self.next_temp();
        self.line(&format!(
            "  {pair} = call {{ i64, i1 }} @llvm.{intrinsic}.with.overflow.i64(i64 {}, i64 {})",
            literal(lhs),
            literal(rhs)
        ));
        let overflow = self.next_temp();
        self.line(&format!(
            "  {overflow} = extractvalue {{ i64, i1 }} {pair}, 1"
        ));
        self.trap_if(&overflow, 0, span);
        self.line(&format!(
            "  %r{dest} = extractvalue {{ i64, i1 }} {pair}, 0"
        ));
    }

    pub(crate) fn remainder(&mut self, dest: usize, lhs: &Value, rhs: &Value, span: Span) {
        let divisor = literal(rhs);

        if self.checks {
            let zero = self.next_temp();
            self.line(&format!("  {zero} = icmp eq i64 {divisor}, 0"));
            self.trap_if(&zero, 2, span);
        }

        let negative = self.next_temp();
        self.line(&format!("  {negative} = icmp eq i64 {divisor}, -1"));
        let safe = self.next_temp();
        self.line(&format!(
            "  {safe} = select i1 {negative}, i64 1, i64 {divisor}"
        ));
        self.line(&format!(
            "  %r{dest} = srem i64 {}, {safe}",
            literal(lhs)
        ));
    }

    pub(crate) fn divide(&mut self, dest: usize, lhs: &Value, rhs: &Value, span: Span) {
        let divisor = literal(rhs);

        if self.checks {
            let zero = self.next_temp();
            self.line(&format!("  {zero} = fcmp oeq double {divisor}, 0.0"));
            self.trap_if(&zero, 1, span);
        }
        self.line(&format!(
            "  %r{dest} = fdiv double {}, {divisor}",
            literal(lhs)
        ));
    }

    pub(crate) fn to_int(&mut self, dest: usize, value: &Value, span: Span) {
        let truncated = self.next_temp();
        self.line(&format!(
            "  {truncated} = call double @llvm.trunc.f64(double {})",
            literal(value)
        ));

        if !self.checks {
            return self.line(&format!("  %r{dest} = fptosi double {truncated} to i64"));
        }

        let unordered = self.next_temp();
        self.line(&format!(
            "  {unordered} = fcmp uno double {truncated}, {truncated}"
        ));
        let low = self.next_temp();
        self.line(&format!(
            "  {low} = fcmp olt double {truncated}, 0xC3E0000000000000"
        ));
        let high = self.next_temp();
        self.line(&format!(
            "  {high} = fcmp oge double {truncated}, 0x43E0000000000000"
        ));
        let partial = self.next_temp();
        self.line(&format!("  {partial} = or i1 {unordered}, {low}"));
        let bad = self.next_temp();
        self.line(&format!("  {bad} = or i1 {partial}, {high}"));
        self.trap_if(&bad, 3, span);
        self.line(&format!(
            "  %r{dest} = fptosi double {truncated} to i64"
        ));
    }
}
