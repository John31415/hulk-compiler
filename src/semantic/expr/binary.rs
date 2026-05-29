use crate::ast::{BinaryOp, BinaryOpKind, Expr};
use crate::diagnostics::Diagnostic;
use crate::lexer::span::Span;
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_binary(&mut self, left: &Expr, op: &BinaryOp, right: &Expr) -> TypeId {
        let left_type = self.check_expr(left);
        let right_type = self.check_expr(right);
        let number_type = self.resolve_builtin("Number");
        let string_type = self.resolve_builtin("String");
        let boolean_type = self.resolve_builtin("Boolean");
        match op.node {
            BinaryOpKind::Add
            | BinaryOpKind::Sub
            | BinaryOpKind::Mul
            | BinaryOpKind::Div
            | BinaryOpKind::Pow => {
                self.enforce_type(left_type, number_type, left.span, op);
                self.enforce_type(right_type, number_type, right.span, op);
                number_type
            }

            BinaryOpKind::Less
            | BinaryOpKind::Greater
            | BinaryOpKind::LessEqual
            | BinaryOpKind::GreaterEqual => {
                self.enforce_type(left_type, number_type, left.span, op);
                self.enforce_type(right_type, number_type, right.span, op);
                boolean_type
            }

            BinaryOpKind::And | BinaryOpKind::Or => {
                self.enforce_type(left_type, boolean_type, left.span, op);
                self.enforce_type(right_type, boolean_type, right.span, op);
                boolean_type
            }

            BinaryOpKind::Concat | BinaryOpKind::ConcatSpace => {
                if left_type != string_type && right_type != string_type {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "Operator '{}' requires at least one String operand, but found left: '{}', right: '{}'",
                            op.node,
                            self.ctx.types.get(left_type).name,
                            self.ctx.types.get(right_type).name,
                        ),
                        left.span.union(&right.span),
                    ));
                }
                string_type
            }

            BinaryOpKind::DoubleEqual | BinaryOpKind::NotEqual => {
                if !self.ctx.types.is_subtype_of(left_type, right_type)
                    && !self.ctx.types.is_subtype_of(right_type, left_type)
                {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "Cannot compare completely unrelated types: '{}' and '{}'",
                            self.ctx.types.get(left_type).name,
                            self.ctx.types.get(right_type).name,
                        ),
                        left.span.union(&right.span),
                    ));
                }
                boolean_type
            }
        }
    }

    pub fn check_is(&mut self, expr: &Expr, type_name: &str, span: Span) -> TypeId {
        let expr_type = self.check_expr(expr);
        let bool_type = self.resolve_builtin("Boolean");
        match self.ctx.types.resolve(type_name) {
            Some(target_type) => {
                if !self.ctx.types.is_subtype_of(expr_type, target_type)
                    && !self.ctx.types.is_subtype_of(target_type, expr_type)
                {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "Expression of type '{}' can never be an instance of '{}'",
                            self.ctx.types.get(expr_type).name,
                            type_name,
                        ),
                        span,
                    ));
                }
            }
            None => {
                self.diagnostics.push(Diagnostic::error(
                    format!("Type '{}' does not exist in the current scope", type_name),
                    span,
                ));
            }
        }
        bool_type
    }

    pub fn check_as(&mut self, expr: &Expr, type_name: &str, span: Span) -> TypeId {
        let expr_type = self.check_expr(expr);
        match self.ctx.types.resolve(type_name) {
            Some(target_type) => {
                if !self.ctx.types.is_subtype_of(expr_type, target_type)
                    && !self.ctx.types.is_subtype_of(target_type, expr_type)
                {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "Cannot cast expression of type '{}' to completely unrelated type '{}'",
                            self.ctx.types.get(expr_type).name,
                            type_name,
                        ),
                        span,
                    ));
                }
                target_type
            }
            None => {
                self.diagnostics.push(Diagnostic::error(
                    format!("Type '{}' does not exist in the current scope", type_name),
                    span,
                ));
                self.ctx.types.resolve("Object").unwrap()
            }
        }
    }

    fn enforce_type(&mut self, current: TypeId, expected: TypeId, span: Span, op: &BinaryOp) {
        if current != expected {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "Operator '{}' expects type '{}', but found '{}'",
                    op.node,
                    self.ctx.types.get(expected).name,
                    self.ctx.types.get(current).name
                ),
                span,
            ));
        }
    }
}
