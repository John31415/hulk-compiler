use crate::ast::{Expr, UnaryOp, UnaryOpKind};
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_unary(&mut self, op: &UnaryOp, expr: &Expr) -> TypeId {
        let inner_type = self.check_expr(expr);
        let number_type = self.resolve_builtin("Number");
        let boolean_type = self.resolve_builtin("Boolean");
        match op.node {
            UnaryOpKind::Neg => {
                if inner_type != number_type {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::InvalidUnaryOperation {
                                operator: "-".to_string(),
                                operand: self.ctx.types.get(inner_type).name.clone(),
                            },
                            expr.span,
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
                            expr.span,
                        )
                        .into(),
                    );
                }
                boolean_type
            }
        }
    }
}
