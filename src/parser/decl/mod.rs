pub mod function_decl;
pub mod protocol_decl;
pub mod type_decl;

use crate::ast::{Decl, Expr};
use crate::lexer::Token;
use crate::parser::decl::{function_decl::*, protocol_decl::*, type_decl::*};
use chumsky::{error::Rich, prelude::*};

pub fn decl_parser<'src>(
    expr: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Decl, extra::Err<Rich<'src, Token>>> {
    let functions = function_decl_parser(expr.clone());
    let types = type_decl_parser(expr.clone());
    let protocols = protocol_decl_parser();
    choice((functions, types, protocols))
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{decl::decl_parser, expr::expr_parser, test_utils::tokenize};
    use chumsky::{Parser, prelude::*};

    #[test]
    fn parser_decl_function_fail() {
        let source = "
        function f() = 1;
        ";

        let parser = decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        assert!(parser.check(&tokens).into_result().is_err())
    }

    #[test]
    fn parser_decl_type_fail() {
        let source = "
        type Point {
            x;
        }
        ";

        let parser = decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        assert!(parser.check(&tokens).into_result().is_err())
    }

    #[test]
    fn parser_decl_protocol_extends_fail() {
        let source = "
        protocol P extends {
            p(): A;
        }
        ";

        let parser = decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        assert!(parser.check(&tokens).into_result().is_err())
    }

    #[test]
    fn parser_decl_protocol_method_fail() {
        let source = "
        interface P {
            p();
        }
        ";

        let parser = decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        assert!(parser.check(&tokens).into_result().is_err())
    }
}
