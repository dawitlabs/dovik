use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast;
use crate::diagnostic::{self, Diagnostic};
use crate::error::{ModuleError, ModuleErrorKind};
use crate::source::SourceMap;
use crate::token::Span;
use crate::{lexer, parser};

pub struct Unit {
    pub path: PathBuf,
    pub ast: ast::Program,
    pub imports: Vec<usize>,
}

pub struct Graph {
    pub units: Vec<Unit>,
    pub sources: SourceMap,
}

impl Graph {
    pub fn entry(&self) -> usize {
        self.units.len().saturating_sub(1)
    }
}

enum State {
    Visiting,
    Done(usize),
}

struct Loader {
    sources: SourceMap,
    units: Vec<Unit>,
    state: HashMap<PathBuf, State>,
    stack: Vec<PathBuf>,
    diagnostics: Vec<Diagnostic>,
}

pub struct Loaded {
    pub graph: Graph,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn unit_graph(path: &str, text: &str, ast: ast::Program) -> Graph {
    Graph {
        sources: SourceMap::single(path, text),
        units: vec![Unit {
            path: PathBuf::from(path),
            ast,
            imports: Vec::new(),
        }],
    }
}

pub fn load(entry: &Path) -> Loaded {
    let mut loader = Loader {
        sources: SourceMap::default(),
        units: Vec::new(),
        state: HashMap::new(),
        stack: Vec::new(),
        diagnostics: Vec::new(),
    };

    let canonical = entry.canonicalize().unwrap_or_else(|_| entry.to_path_buf());
    loader.visit(&canonical, entry, None);
    loader.diagnostics.sort_by_key(|item| item.span.start);

    Loaded {
        graph: Graph {
            units: loader.units,
            sources: loader.sources,
        },
        diagnostics: loader.diagnostics,
    }
}

impl Loader {
    fn visit(&mut self, canonical: &Path, display: &Path, at: Option<Span>) -> Option<usize> {
        match self.state.get(canonical) {
            Some(State::Done(index)) => return Some(*index),
            Some(State::Visiting) => {
                self.report_cycle(canonical, at);
                return None;
            }
            None => {}
        }

        let text = match std::fs::read_to_string(canonical) {
            Ok(text) => text,
            Err(err) => {
                self.unreadable(display, err.to_string(), at);
                return None;
            }
        };

        self.state.insert(canonical.to_path_buf(), State::Visiting);
        self.stack.push(canonical.to_path_buf());

        let mut unit = self.parse_unit(display.to_path_buf(), text);
        let directory = canonical.parent().unwrap_or(Path::new(".")).to_path_buf();

        for import in &unit.ast.imports {
            let joined = directory.join(&import.path);
            let Ok(resolved) = joined.canonicalize() else {
                self.unreadable(&joined, "no such file".to_string(), Some(import.path_span));
                continue;
            };

            match self.visit(&resolved, &joined, Some(import.path_span)) {
                Some(index) => unit.imports.push(index),
                None => continue,
            }
        }

        self.stack.pop();

        let index = self.units.len();
        self.units.push(unit);
        self.state.insert(canonical.to_path_buf(), State::Done(index));

        Some(index)
    }

    fn parse_unit(&mut self, path: PathBuf, text: String) -> Unit {
        let base = self.sources.add(path.display().to_string(), text.clone());
        let (tokens, lex_errors) = lexer::lex_at(&text, base);

        if !lex_errors.is_empty() {
            self.diagnostics.extend(diagnostic::from_lex(&lex_errors));

            return Unit {
                path,
                ast: ast::Program {
                    imports: Vec::new(),
                    statements: Vec::new(),
                },
                imports: Vec::new(),
            };
        }

        let (parsed, parse_errors) = parser::parse(self.sources.text(), &tokens);
        self.diagnostics.extend(diagnostic::from_parse(&parse_errors));

        Unit {
            path,
            ast: parsed,
            imports: Vec::new(),
        }
    }

    fn unreadable(&mut self, path: &Path, reason: String, at: Option<Span>) {
        let error = ModuleError {
            kind: ModuleErrorKind::Unreadable {
                path: path.display().to_string(),
                reason,
            },
            span: at.unwrap_or(Span::new(0, 0)),
        };
        self.diagnostics.push(diagnostic::from_module(&error));
    }

    fn report_cycle(&mut self, canonical: &Path, at: Option<Span>) {
        let start = self
            .stack
            .iter()
            .position(|path| path == canonical)
            .unwrap_or(0);

        let mut names: Vec<String> = self.stack[start..]
            .iter()
            .map(|path| short(path))
            .collect();
        names.push(short(canonical));

        let error = ModuleError {
            kind: ModuleErrorKind::Cycle(names),
            span: at.unwrap_or(Span::new(0, 0)),
        };
        self.diagnostics.push(diagnostic::from_module(&error));
    }
}

fn short(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
