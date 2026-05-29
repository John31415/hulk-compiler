use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_block(&mut self, expressions: &Vec<Expr>, span: Span) -> TypeId {
        if expressions.is_empty() {
            return self.ctx.types.resolve("Object").unwrap();
        }
        self.ctx.push_scope();
        let mut last_type_id = self.ctx.types.resolve("Object").unwrap();
        for (i, expr) in expressions.iter().enumerate() {
            let expr_type = self.check_expr(expr);
            if i == expressions.len() - 1 {
                last_type_id = expr_type;
            }
        }
        self.ctx.pop_scope();
        last_type_id
    }
}
