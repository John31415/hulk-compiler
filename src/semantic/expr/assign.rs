use crate::ast::{Expr, ExprKind};
use crate::diagnostics::Diagnostic;
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_assign(&mut self, target: &Expr, value: &Expr) -> TypeId {
        let target_type = match &target.node {
            ExprKind::Variable(name) => {
                if name == "self" {
                    self.diagnostics.push(Diagnostic::error(
                        format!("Cannot assign to 'self' because it is read-only"),
                        target.span,
                    ));
                    self.check_expr(value);
                    return self.ctx.types.resolve("Object").unwrap();
                }
                self.check_variable(name, target.span)
            }
            ExprKind::PropertyAccess { .. } => self.check_expr(target),
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    format!("Left-hand side of assignment must be a variable or a property"),
                    target.span,
                ));
                self.ctx.types.resolve("Object").unwrap()
            }
        };
        let value_type = self.check_expr(value);
        if !self.ctx.types.is_subtype_of(value_type, target_type) {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "Cannot assign type '{}' to a target of type '{}'",
                    self.ctx.types.get(value_type).name,
                    self.ctx.types.get(target_type).name,
                ),
                value.span,
            ));
        }
        value_type
    }
}
