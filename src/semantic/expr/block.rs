use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::hir::{TypedExpr, TypedExprKind};

impl SemanticAnalyzer {
    pub fn analyze_block(&mut self, expressions: &Vec<Expr>, span: Span) -> TypedExpr {
        self.ctx.push_scope();
        let mut last_type_id = self.ctx.types.resolve("Object").unwrap();
        let mut typed_expressions = Vec::new();
        for expr in expressions {
            let expr_type = self.analyze_expr(expr);
            last_type_id = expr_type.ty;
            typed_expressions.push(expr_type);
        }
        self.ctx.pop_scope();
        TypedExpr::new(TypedExprKind::Block(typed_expressions), last_type_id, span)
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_assign_ok() {
        let source = r#"
{
    {
       42;
    };
    {
       42;
    };
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 0);
    }
}
