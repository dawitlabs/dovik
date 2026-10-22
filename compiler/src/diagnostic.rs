use std::fmt::Write;

use crate::error::{LexError, ModuleError, ParseError, ResolveError, TypeError};
use crate::source::SourceMap;
use crate::token::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn render(&self, sources: &SourceMap) -> String {
        let (path, line, column) = sources.locate(self.span.start);
        format!(
            "{path}:{line}:{column}: error[{}]: {}",
            self.code, self.message
        )
    }

    pub fn to_json(&self, sources: &SourceMap) -> String {
        let (path, line, column) = sources.locate(self.span.start);
        let mut out = String::new();
        let _ = write!(
            out,
            "{{\"severity\":\"error\",\"code\":\"{}\",\"file\":{},\"line\":{line},\"column\":{column},\"start\":{},\"end\":{},\"message\":{}}}",
            self.code,
            quote(path),
            self.span.start,
            self.span.end,
            quote(&self.message)
        );
        out
    }
}

pub fn from_lex(errors: &[LexError]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|error| Diagnostic {
            code: error.kind.code(),
            message: error.kind.to_string(),
            span: error.span,
        })
        .collect()
}

pub fn from_module(error: &ModuleError) -> Diagnostic {
    Diagnostic {
        code: error.kind.code(),
        message: error.kind.to_string(),
        span: error.span,
    }
}

pub fn from_parse(errors: &[ParseError]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|error| Diagnostic {
            code: error.kind.code(),
            message: error.kind.to_string(),
            span: error.span,
        })
        .collect()
}

pub fn from_resolve(errors: &[ResolveError]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|error| Diagnostic {
            code: error.kind.code(),
            message: error.kind.to_string(),
            span: error.span,
        })
        .collect()
}

pub fn from_type(errors: &[TypeError]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|error| Diagnostic {
            code: error.kind.code(),
            message: error.kind.to_string(),
            span: error.span,
        })
        .collect()
}

pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');

    for scalar in text.chars() {
        match scalar {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control < ' ' => {
                let _ = write!(out, "\\u{:04x}", control as u32);
            }
            other => out.push(other),
        }
    }

    out.push('"');
    out
}
