use crate::ast::{Expr, UnaryOp, UnaryOpKind};
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};

impl SemanticAnalyzer {
    pub fn analyze_unary(&mut self, op: &UnaryOp, expr: &Expr, span: Span) -> TypedExpr {
        let inner = self.analyze_expr(expr);
        let inner_type = inner.ty;
        let number_type = self.resolve_builtin("Number");
        let boolean_type = self.resolve_builtin("Boolean");
        let type_id = match op.node {
            UnaryOpKind::Neg => {
                if inner_type != number_type {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::InvalidUnaryOperation {
                                operator: "-".to_string(),
                                operand: self.ctx.types.get(inner_type).name.clone(),
                            },
                            span,
                        )
                        .into(),
                    );
                }
                number_type
            }
            UnaryOpKind::Not => {
                if inner_type != boolean_type {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::InvalidUnaryOperation {
                                operator: "!".to_string(),
                                operand: self.ctx.types.get(inner_type).name.clone(),
                            },
                            span,
                        )
                        .into(),
                    );
                }
                boolean_type
            }
        };
        TypedExpr::new(
            TypedExprKind::Unary {
                op: op.clone(),
                expr: Box::new(inner),
            },
            type_id,
            span,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_unary() {
        let source = r#"
{
    -true;
    !1;
}
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 2);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidUnaryOperation {
                operator: "-".to_string(),
                operand: "Boolean".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::InvalidUnaryOperation {
                operator: "!".to_string(),
                operand: "Number".to_string()
            }
        );
    }
}
