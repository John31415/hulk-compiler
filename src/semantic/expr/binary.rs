use crate::ast::{BinaryOp, BinaryOpKind, Expr};
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{TypedExpr, TypedExprKind};
use crate::semantic::{SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn analyze_binary(
        &mut self,
        left: &Expr,
        op: &BinaryOp,
        right: &Expr,
        span: Span,
    ) -> TypedExpr {
        let left_type = self.analyze_expr(left);
        let right_type = self.analyze_expr(right);
        let number_type = self.resolve_builtin("Number");
        let string_type = self.resolve_builtin("String");
        let boolean_type = self.resolve_builtin("Boolean");
        let result_type = match op.node {
            BinaryOpKind::Add
            | BinaryOpKind::Sub
            | BinaryOpKind::Mul
            | BinaryOpKind::Div
            | BinaryOpKind::Pow => {
                self.enforce_type(left_type.ty, number_type, left.span, op);
                self.enforce_type(right_type.ty, number_type, right.span, op);
                number_type
            }

            BinaryOpKind::Less
            | BinaryOpKind::Greater
            | BinaryOpKind::LessEqual
            | BinaryOpKind::GreaterEqual => {
                self.enforce_type(left_type.ty, number_type, left.span, op);
                self.enforce_type(right_type.ty, number_type, right.span, op);
                boolean_type
            }

            BinaryOpKind::And | BinaryOpKind::Or => {
                self.enforce_type(left_type.ty, boolean_type, left.span, op);
                self.enforce_type(right_type.ty, boolean_type, right.span, op);
                boolean_type
            }

            BinaryOpKind::Concat | BinaryOpKind::ConcatSpace => {
                if left_type.ty != string_type && right_type.ty != string_type {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::InvalidBinaryOperation {
                                operator: op.node.to_string(),
                                left: self.ctx.types.get(left_type.ty).name.clone(),
                                right: self.ctx.types.get(right_type.ty).name.clone(),
                            },
                            left.span.union(&right.span),
                        )
                        .into(),
                    );
                }
                if (left_type.ty != string_type && left_type.ty != number_type)
                    || (right_type.ty != string_type && right_type.ty != number_type)
                {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::InvalidBinaryOperation {
                                operator: op.node.to_string(),
                                left: self.ctx.types.get(left_type.ty).name.clone(),
                                right: self.ctx.types.get(right_type.ty).name.clone(),
                            },
                            left.span.union(&right.span),
                        )
                        .into(),
                    );
                }
                string_type
            }

            BinaryOpKind::DoubleEqual | BinaryOpKind::NotEqual => {
                if !self.ctx.types.is_subtype_of(left_type.ty, right_type.ty)
                    && !self.ctx.types.is_subtype_of(right_type.ty, left_type.ty)
                {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::IncomparableTypes {
                                left: self.ctx.types.get(left_type.ty).name.clone(),
                                right: self.ctx.types.get(right_type.ty).name.clone(),
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
            TypedExprKind::Binary {
                left_expr: Box::new(left_type),
                op: op.clone(),
                right_expr: Box::new(right_type),
            },
            result_type,
            span,
        )
    }

    pub fn analyze_is(&mut self, expr: &Expr, type_name: &str, span: Span) -> TypedExpr {
        let expr_type = self.analyze_expr(expr);
        let bool_type = self.resolve_builtin("Boolean");
        let resolved_target_type = match self.ctx.types.resolve(type_name) {
            Some(target_type) => {
                if !self.ctx.types.is_subtype_of(expr_type.ty, target_type)
                    && !self.ctx.types.is_subtype_of(target_type, expr_type.ty)
                {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::ImpossibleTypeCheck {
                                expr: self.ctx.types.get(expr_type.ty).name.clone(),
                                target: type_name.to_string(),
                            },
                            span,
                        )
                        .into(),
                    );
                }
                target_type
            }
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UnknownType {
                            name: type_name.to_string(),
                        },
                        span,
                    )
                    .into(),
                );
                self.resolve_builtin("Object")
            }
        };
        TypedExpr::new(
            TypedExprKind::Is {
                expr: Box::new(expr_type),
                target_type: resolved_target_type,
            },
            bool_type,
            span,
        )
    }

    pub fn analyze_as(&mut self, expr: &Expr, type_name: &str, span: Span) -> TypedExpr {
        let expr_type = self.analyze_expr(expr);
        let resolved_target_type = match self.ctx.types.resolve(type_name) {
            Some(target_type) => {
                if !self.ctx.types.is_subtype_of(expr_type.ty, target_type)
                    && !self.ctx.types.is_subtype_of(target_type, expr_type.ty)
                {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::InvalidCast {
                                from: self.ctx.types.get(expr_type.ty).name.clone(),
                                to: type_name.to_string(),
                            },
                            span,
                        )
                        .into(),
                    );
                }
                target_type
            }
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UnknownType {
                            name: type_name.to_string(),
                        },
                        span,
                    )
                    .into(),
                );
                self.resolve_builtin("Object")
            }
        };
        TypedExpr::new(
            TypedExprKind::As {
                expr: Box::new(expr_type),
                target_type: resolved_target_type,
            },
            resolved_target_type,
            span,
        )
    }

    fn enforce_type(&mut self, current: TypeId, expected: TypeId, span: Span, op: &BinaryOp) {
        if current != expected {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidOperatorOperand {
                        operator: op.node.to_string(),
                        expected: self.ctx.types.get(expected).name.clone(),
                        found: self.ctx.types.get(current).name.clone(),
                    },
                    span,
                )
                .into(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_binary_err() {
        let source = r#"
{
    22 + "a";
    2 <= true;
    3 & "a";
    2 @ true;
    2 == "a";
    2 is String;
    2 is John;
    2 as String;
    2 as John;
}
    "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 10);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidOperatorOperand {
                operator: "+".to_string(),
                expected: "Number".to_string(),
                found: "String".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[1].kind,
            SemanticErrorKind::InvalidOperatorOperand {
                operator: "<=".to_string(),
                expected: "Number".to_string(),
                found: "Boolean".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[2].kind,
            SemanticErrorKind::InvalidOperatorOperand {
                operator: "&".to_string(),
                expected: "Boolean".to_string(),
                found: "Number".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[3].kind,
            SemanticErrorKind::InvalidOperatorOperand {
                operator: "&".to_string(),
                expected: "Boolean".to_string(),
                found: "String".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[4].kind,
            SemanticErrorKind::InvalidBinaryOperation {
                operator: "@".to_string(),
                left: "Number".to_string(),
                right: "Boolean".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[5].kind,
            SemanticErrorKind::IncomparableTypes {
                left: "Number".to_string(),
                right: "String".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[6].kind,
            SemanticErrorKind::ImpossibleTypeCheck {
                expr: "Number".to_string(),
                target: "String".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[7].kind,
            SemanticErrorKind::UnknownType {
                name: "John".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[8].kind,
            SemanticErrorKind::InvalidCast {
                from: "Number".to_string(),
                to: "String".to_string()
            }
        );
        assert_eq!(
            analyzer.diagnostics[9].kind,
            SemanticErrorKind::UnknownType {
                name: "John".to_string()
            }
        );
    }
}
