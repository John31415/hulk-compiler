use crate::{
    diagnostics::{Diagnostic, Label},
    lexer::{Span, Token},
};
use chumsky::error::Rich;

pub fn span_from_token_slice(tokens: &[Token]) -> Span {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => Span::new(first.span.start, last.span.end),
        _ => Span::new(0, 0),
    }
}

pub fn rich_to_diagnostic(error: Rich<Token>, tokens: &[Token]) -> Diagnostic {
    let error_range = error.span().into_range();
    let error_tokens = if error_range.start < tokens.len() {
        &tokens[error_range.start..error_range.end.min(tokens.len())]
    } else {
        &[]
    };
    let span = if let Some(token) = error.found() {
        token.span
    } else if !error_tokens.is_empty() {
        span_from_token_slice(error_tokens)
    } else {
        Span::new(0, 0)
    };
    let found = error
        .found()
        .map(|t| format!("{:?}", t.kind))
        .unwrap_or_else(|| "end of input".to_string());
    let expected = {
        let items: Vec<String> = error.expected().map(|e| format!("{:?}", e)).collect();
        if items.is_empty() {
            None
        } else {
            Some(items.join(", "))
        }
    };
    let message = if found == "end of input" {
        match &expected {
            Some(exp) => format!("unexpected end of input, expected one of: {exp}"),
            None => "unexpected end of input".to_string(),
        }
    } else {
        match &expected {
            Some(exp) => format!("unexpected token `{found}`, expected one of: {exp}"),
            None => format!("unexpected token `{found}`"),
        }
    };
    let mut diagnostic = Diagnostic::error(message, span)
        .with_label(Label::new(format!("found `{found}` here"), span));
    if let Some(exp) = expected {
        diagnostic = diagnostic.with_note(format!("expected one of: {exp}"));
    }
    diagnostic
}
