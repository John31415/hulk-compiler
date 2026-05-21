pub mod assign;
pub mod binary;
pub mod block;
pub mod control_flow;
pub mod let_expr;
pub mod new;
pub mod postfix;
pub mod primary;
pub mod unary;

use crate::ast::Expr;
use crate::lexer::{Token, TokenKind};
use crate::parser::expr::assign::*;
use crate::parser::expr::binary::*;
use crate::parser::expr::block::*;
use crate::parser::expr::control_flow::*;
use crate::parser::expr::let_expr::*;
use crate::parser::expr::new::new_parser;
use crate::parser::expr::postfix::*;
use crate::parser::expr::primary::*;
use crate::parser::expr::unary::*;
use chumsky::{error::Rich, prelude::*};

pub fn expr_parser<'src>() -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>>
{
    recursive(|expr| {
        let paren_expr = expr.clone().delimited_by(
            select_ref! {Token{kind: TokenKind::LParen, ..} => ()},
            select_ref! {Token{kind: TokenKind::RParen, ..} => ()},
        );
        let block = block_parser(expr.clone());
        let primary = choice((primary_parser(), paren_expr, block)).boxed();
        let atom = choice((new_parser(expr.clone()).boxed(), primary)).boxed();
        let postfix = postfix_parser(expr.clone(), atom).boxed();
        let unary = unary_parser(postfix).boxed();
        let exponent = exponent_parser(unary).boxed();
        let product = product_parser(exponent).boxed();
        let sum = sum_at_parser(product).boxed();
        let comparison = comparison_parser(sum).boxed();
        let is = is_parser(comparison).boxed();
        let as_expr = as_expr_parser(is).boxed();
        let equality = equality_parser(as_expr).boxed();
        let logical_and = logical_and_parser(equality).boxed();
        let logical_or = logical_or_parser(logical_and).boxed();
        let if_expr = if_expr_parser(logical_or).boxed();
        let while_expr = while_expr_parser(if_expr).boxed();
        let for_expr = for_expr_parser(while_expr).boxed();
        let let_expr = let_expr_parser(for_expr).boxed();
        let assign = assign_parser(let_expr).boxed();
        assign
    })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{expr::expr_parser, test_utils::tokenize};
    use chumsky::{Parser, prelude::*};
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_expr_control_flow() {
        let source = "
        for (x in range(1, 5)) {
            if (true) {
                while (true) {
                    x := x + 1;
                };
            } 
            elif (false) 2
            else {
                print(x);
            };
        }
        ";

        let parser = expr_parser();

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_expr_multiple_block() {
        let source = "
        {
            \" blessings \" @ (a * 2 + b * 3 ^ c) is String;
            let a = 1, b: Number = 2 in
                true & (!a | !b) == (a & b);
        }
        ";

        let parser = expr_parser();

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_expr_multiple_recursive() {
        let source = "
        let a = {
            {
               {
                    2;
               };
            };
        } in (a := a ^ ( 2 + 3 * 2))
        ";

        let parser = expr_parser();

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}

