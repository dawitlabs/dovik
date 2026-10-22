use super::spacing::{canonical_type_name, keyword, Context};
use crate::token::{Token, TokenKind};

pub(super) fn comment(text: &str) -> String {
    let body = text.trim_start_matches('#').trim_end();

    if body.is_empty() {
        return "#".to_string();
    }

    match body.starts_with(' ') {
        true => format!("#{body}"),
        false => format!("# {body}"),
    }
}

pub(super) fn line(tokens: &[&Token], source: &str) -> String {
    let mut out = String::new();
    let mut context = Context::default();

    for (index, token) in tokens.iter().enumerate() {
        let previous = index.checked_sub(1).map(|before| &tokens[before].kind);
        let next = tokens.get(index + 1).map(|after| &after.kind);

        if let Some(previous) = previous {
            if context.needs_space(previous, &token.kind) {
                out.push(' ');
            }
        }

        if context.in_type && token.kind == TokenKind::GtEq {
            out.push_str("> =");
            context.close_angle();
            context.in_type = false;
            continue;
        }

        out.push_str(&text(token, source));
        context.advance(&token.kind, next);
    }

    out
}

fn text(token: &Token, source: &str) -> String {
    match &token.kind {
        TokenKind::Identifier => canonical_type_name(token.text(source)),
        TokenKind::Int(value) => value.to_string(),
        TokenKind::Float(_) | TokenKind::Str(_) => token.text(source).to_string(),
        other => keyword(other).to_string(),
    }
}
