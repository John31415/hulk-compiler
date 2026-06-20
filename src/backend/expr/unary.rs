use super::{Backend, BackendError, BackendResult};
use crate::ast::UnaryOpKind;
use crate::semantic::{
    SemanticAnalyzer,
    hir::{TypedExpr, TypedExprKind},
};
use inkwell::values::BasicValueEnum;

impl<'ctx> Backend<'ctx> {
    pub fn compile_unary(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        let (op, operand) = match &expr.node {
            TypedExprKind::Unary { op, expr } => (op, expr),
            _ => return Err(BackendError::InvalidExpression),
        };
        let val = self.compile_expr(operand, sema)?;
        match op.node {
            UnaryOpKind::Neg => {
                if val.is_int_value() {
                    let int_val = val.into_int_value();
                    let res = self
                        .builder
                        .build_int_neg(int_val, "neg_int")
                        .map_err(|_| BackendError::InvalidExpression)?;
                    Ok(BasicValueEnum::IntValue(res))
                } else if val.is_float_value() {
                    let float_val = val.into_float_value();
                    let res = self
                        .builder
                        .build_float_neg(float_val, "neg_float")
                        .map_err(|_| BackendError::InvalidExpression)?;
                    Ok(BasicValueEnum::FloatValue(res))
                } else {
                    Err(BackendError::InvalidExpression)
                }
            }
            UnaryOpKind::Not => {
                if val.is_int_value() {
                    let int_val = val.into_int_value();
                    let res = self
                        .builder
                        .build_not(int_val, "not_val")
                        .map_err(|_| BackendError::InvalidExpression)?;
                    Ok(BasicValueEnum::IntValue(res))
                } else {
                    Err(BackendError::InvalidExpression)
                }
            }
        }
    }
}
