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
