pub mod function_decl;
pub mod type_decl;

use crate::ast::{Decl, Expr};
use crate::lexer::Token;
use crate::parser::decl::function_decl::*;
use crate::parser::decl::type_decl::*;
use chumsky::{error::Rich, prelude::*};

pub fn decl_parser<'src>(
    expr: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Decl, extra::Err<Rich<'src, Token>>> {
    function_decl_parser(expr.clone()).or(type_decl_parser(expr.clone()))
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
}
