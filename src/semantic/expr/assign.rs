use crate::ast::{Expr, ExprKind};
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_assign(&mut self, target: &Expr, value: &Expr) -> TypeId {
        let target_type = match &target.node {
            ExprKind::Variable(name) => {
                if name == "self" {
                    self.diagnostics.push(
                        SemanticError::new(SemanticErrorKind::InvalidAssignmentTarget, target.span)
                            .into(),
                    );
                    self.check_expr(value);
                    return self.ctx.types.resolve("Object").unwrap();
                }
                self.check_variable(name, target.span)
            }
            ExprKind::PropertyAccess { .. } => self.check_expr(target),
            _ => {
                self.diagnostics.push(
                    SemanticError::new(SemanticErrorKind::InvalidAssignmentTarget, target.span)
                        .into(),
                );
                self.ctx.types.resolve("Object").unwrap()
            }
        };
        let value_type = self.check_expr(value);
        if !self.ctx.types.is_subtype_of(value_type, target_type) {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::TypeMismatch {
                        expected: self.ctx.types.get(target_type).name.clone(),
                        found: self.ctx.types.get(value_type).name.clone(),
                    },
                    value.span,
                )
                .into(),
            );
        }
        value_type
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
        analyzer.analyze_program(
            program.node.decls.as_deref().unwrap_or(&[]),
            Some(&program.node.body),
        );
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
