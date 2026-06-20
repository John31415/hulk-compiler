use inkwell::values::BasicValueEnum;

use crate::semantic::{
    SemanticAnalyzer,
    hir::{TypedExpr, TypedExprKind},
};

use super::{Backend, BackendError, BackendResult};

impl<'ctx> Backend<'ctx> {
    pub fn compile_block(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        if let TypedExprKind::Block(expressions) = &expr.node {
            let mut last_val = None;
            for expression in expressions {
                last_val = Some(self.compile_expr(expression, sema)?);
            }
            return Ok(last_val.unwrap());
        }
        Err(BackendError::InvalidExpression)
    }
}
