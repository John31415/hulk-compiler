use crate::ast::{Decl, DeclKind, ProtocolMethodsKind, Spanned};
use crate::lexer::{Token, TokenKind};
use crate::parser::span_from_token_slice;
use chumsky::{error::Rich, prelude::*};

pub fn protocol_decl_parser<'src>()
-> impl Parser<'src, &'src [Token], Decl, extra::Err<Rich<'src, Token>>> {
    let semi = select_ref! {
        Token {
            kind: TokenKind::Semi,
            ..
        } => ()
    };
    let comma = select_ref! {
        Token {
            kind: TokenKind::Comma,
            ..
        } => ()
    };
    let ident = select_ref! {
        Token {
            kind: TokenKind::Identifier(name),
            ..
        } => name.clone()
    };
    let parents = select_ref! {
        Token {
            kind: TokenKind::Extends,
            ..
        } => ()
    }
    .ignore_then(
        ident
            .clone()
            .separated_by(comma.clone())
            .at_least(1)
            .collect::<Vec<_>>(),
    );
    let type_name = select_ref! {
        Token {
            kind: TokenKind::Colon,
            ..
        } => ()
    }
    .ignore_then(ident.clone());
    let param = ident.then(type_name.clone());
    let params = param
        .separated_by(comma.clone())
        .collect::<Vec<_>>()
        .delimited_by(
            select_ref! { Token { kind: TokenKind::LParen, .. } => () },
            select_ref! { Token { kind: TokenKind::RParen, .. } => () },
        );
    let method = ident
        .clone()
        .then(params.clone())
        .then(type_name.clone())
        .then_ignore(semi.clone())
        .map_with(|((name, params), return_type), e| {
            let span = span_from_token_slice(e.slice());
            Spanned::new(
                ProtocolMethodsKind {
                    name,
                    params,
                    return_type,
                },
                span,
            )
        });
    let methods = method
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .delimited_by(
            select_ref! { Token {kind: TokenKind::LBrace, .. } => () },
            select_ref! { Token {kind: TokenKind::RBrace, .. } => () },
        );
    let protocol_interface = select_ref! {
        Token {
            kind: TokenKind::Protocol,
            ..
        } => ()
    }
    .or(select_ref! {
        Token {
            kind: TokenKind::Interface,
            ..
        } => ()
    });
    protocol_interface
        .clone()
        .ignore_then(ident.clone())
        .then(parents.or_not())
        .then(methods)
        .map_with(|((name, parents), methods), span| {
            let span = span_from_token_slice(span.slice());
            Spanned::new(
                DeclKind::Protocol {
                    name,
                    parents,
                    methods,
                },
                span,
            )
        })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{decl::protocol_decl::*, test_utils::tokenize};
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_decl_protocol() {
        let source = "
interface I extends P1, P2 {
    m3(a: King, b: Pawn): Piece;
}
        ";

        let parser = protocol_decl_parser();

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
