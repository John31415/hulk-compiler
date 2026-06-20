use inkwell::values::BasicValueEnum;

use crate::semantic::{
    SemanticAnalyzer,
    hir::{TypedExpr, TypedExprKind},
};

use super::{Backend, BackendError, BackendResult};

impl<'ctx> Backend<'ctx> {
    pub fn compile_let(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        if let TypedExprKind::Let { name, value, body } = &expr.node {
            let init_val = self.compile_expr(value, sema)?;
            let llvm_ty = init_val.get_type();
            self.push_scope();
            let ptr = self.create_entry_block_alloca(name, llvm_ty);
            self.builder
                .build_store(ptr, init_val)
                .map_err(|_| BackendError::InvalidExpression)?;
            self.insert_local(name, ptr);
            let body_val = self.compile_expr(body, sema)?;
            self.pop_scope();
            return Ok(body_val);
        }
        Err(BackendError::InvalidExpression)
    }
}
