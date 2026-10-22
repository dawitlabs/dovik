mod render;
mod spacing;

use crate::lexer;
use crate::token::{Span, Token, TokenKind};

const INDENT: &str = "    ";

enum Item<'a> {
    Comment { text: String, line: usize },
    Line { tokens: Vec<&'a Token>, depth: usize, line: usize },
}

impl Item<'_> {
    fn line(&self) -> usize {
        match self {
            Self::Comment { line, .. } | Self::Line { line, .. } => *line,
        }
    }
}

pub fn format(source: &str) -> Option<String> {
    let (tokens, comments, errors) = lexer::lex_with_trivia(source);

    if !errors.is_empty() {
        return None;
    }

    let lines = line_index(source);
    let items = collect(&tokens, &comments, source, &lines);
    Some(assemble(&items, source))
}

fn collect<'a>(
    tokens: &'a [Token],
    comments: &[Span],
    source: &str,
    lines: &[usize],
) -> Vec<Item<'a>> {
    let mut items: Vec<Item<'a>> = Vec::new();
    let mut pending: Vec<&'a Token> = Vec::new();
    let mut depth = 0usize;
    let mut next_comment = 0usize;

    for token in tokens {
        match token.kind {
            TokenKind::Indent => depth += 1,
            TokenKind::Dedent => depth = depth.saturating_sub(1),
            TokenKind::Newline | TokenKind::Eof => {
                let boundary = pending
                    .first()
                    .map(|first| first.span.start)
                    .unwrap_or(token.span.start);

                while next_comment < comments.len() && comments[next_comment].start < boundary {
                    let span = comments[next_comment];
                    items.push(Item::Comment {
                        text: render::comment(&source[span.start..span.end]),
                        line: line_of(lines, span.start),
                    });
                    next_comment += 1;
                }

                if !pending.is_empty() {
                    let line = line_of(lines, pending[0].span.start);
                    items.push(Item::Line {
                        tokens: std::mem::take(&mut pending),
                        depth,
                        line,
                    });
                }

                while next_comment < comments.len()
                    && comments[next_comment].start < token.span.start
                {
                    let span = comments[next_comment];
                    items.push(Item::Comment {
                        text: render::comment(&source[span.start..span.end]),
                        line: line_of(lines, span.start),
                    });
                    next_comment += 1;
                }
            }
            _ => pending.push(token),
        }
    }

    for span in comments.iter().skip(next_comment) {
        items.push(Item::Comment {
            text: render::comment(&source[span.start..span.end]),
            line: line_of(lines, span.start),
        });
    }

    items
}

fn assemble(items: &[Item<'_>], source: &str) -> String {
    let mut out = String::new();
    let mut previous: Option<usize> = None;
    let mut index = 0;

    while index < items.len() {
        if let Some(last) = previous {
            if items[index].line() > last + 1 {
                out.push('\n');
            }
        }

        match &items[index] {
            Item::Comment { text, line } => {
                let depth = items
                    .get(index + 1)
                    .and_then(|next| match next {
                        Item::Line { depth, line: next_line, .. } if *next_line == line + 1 => {
                            Some(*depth)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| enclosing_depth(items, index));

                out.push_str(&INDENT.repeat(depth));
                out.push_str(text);
                out.push('\n');
                previous = Some(*line);
                index += 1;
            }
            Item::Line { tokens, depth, line } => {
                out.push_str(&INDENT.repeat(*depth));
                out.push_str(&render::line(tokens, source));

                let mut last = *line;
                let mut trailing = index + 1;

                if let Some(Item::Comment { text, line: comment_line }) = items.get(trailing) {
                    if *comment_line == last {
                        out.push_str("  ");
                        out.push_str(text);
                        last = *comment_line;
                        trailing += 1;
                    }
                }

                out.push('\n');
                previous = Some(last);
                index = trailing;
            }
        }
    }

    out
}

fn enclosing_depth(items: &[Item<'_>], index: usize) -> usize {
    items[..index]
        .iter()
        .rev()
        .find_map(|item| match item {
            Item::Line { depth, .. } => Some(*depth),
            _ => None,
        })
        .unwrap_or(0)
}

fn line_index(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
    starts
}

fn line_of(lines: &[usize], offset: usize) -> usize {
    lines.partition_point(|start| *start <= offset)
}
