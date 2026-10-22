use std::io::{self, Write};

use crate::diagnostic::Diagnostic;

pub struct Report<'a> {
    pub path: &'a str,
    pub source: &'a str,
    pub json: bool,
}

impl Report<'_> {
    pub fn emit(&self, diagnostics: &[Diagnostic]) {
        let mut err = io::stderr().lock();

        for diagnostic in diagnostics {
            let line = match self.json {
                true => diagnostic.to_json(self.path, self.source),
                false => diagnostic.render(self.path, self.source),
            };

            if writeln!(err, "{line}").is_err() {
                return;
            }
        }
    }

    pub fn note(&self, message: &str) {
        if self.json {
            eprintln!(
                "{{\"severity\":\"error\",\"code\":\"DVK0001\",\"file\":{},\"message\":{}}}",
                crate::diagnostic::quote(self.path),
                crate::diagnostic::quote(message)
            );
            return;
        }

        eprintln!("{}: {message}", self.path);
    }
}

pub fn write_stdout(lines: impl Iterator<Item = String>) {
    let mut out = io::stdout().lock();

    for line in lines {
        if writeln!(out, "{line}").is_err() {
            return;
        }
    }
}
