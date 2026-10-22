use crate::ir::CompareOp;

pub(crate) fn integer_predicate(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "eq",
        CompareOp::Ne => "ne",
        CompareOp::Lt => "slt",
        CompareOp::Gt => "sgt",
        CompareOp::Le => "sle",
        CompareOp::Ge => "sge",
    }
}

pub(crate) fn float_predicate(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "oeq",
        CompareOp::Ne => "une",
        CompareOp::Lt => "olt",
        CompareOp::Gt => "ogt",
        CompareOp::Le => "ole",
        CompareOp::Ge => "oge",
    }
}
