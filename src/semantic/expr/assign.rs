use crate::ast::{Expr, ExprKind};
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};

impl SemanticAnalyzer {
    pub fn analyze_assign(&mut self, target: &Expr, value: &Expr, span: Span) -> TypedExpr {
        let value_type = self.analyze_expr(value);
        let target_expr = self.analyze_expr(target);
        let target_type = match &target.node {
            ExprKind::Variable(name) => {
                if name == "self" {
                    self.diagnostics.push(
                        SemanticError::new(SemanticErrorKind::InvalidAssignmentTarget, target.span)
                            .into(),
                    );
                    self.resolve_builtin("Object")
                } else {
                    target_expr.ty
                }
            }
            ExprKind::PropertyAccess { .. } => target_expr.ty,
            _ => {
                self.diagnostics.push(
                    SemanticError::new(SemanticErrorKind::InvalidAssignmentTarget, target.span)
                        .into(),
                );
                self.resolve_builtin("Object")
            }
        };

        if !self.ctx.types.is_subtype_of(value_type.ty, target_type) {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::TypeMismatch {
                        expected: self.ctx.types.get(target_type).name.clone(),
                        found: self.ctx.types.get(value_type.ty).name.clone(),
                    },
                    value.span,
                )
                .into(),
            );
        }
        let type_id = value_type.ty;
        TypedExpr::new(
            TypedExprKind::Assign {
                target: Box::new(target_expr),
                value: Box::new(value_type),
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
    fn semantic_unit_test_assign_err() {
        let source = r#"
type A {
    // ...
    f() {
        self := new A(); // <-- Semantic error, `self` is not a valid assignment target
        let a: Number = 1 in { a := true; };
    }
}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 2);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidAssignmentTarget
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::TypeMismatch {
                expected: "Number".to_string(),
                found: "Boolean".to_string()
            }
        );
    }
}
