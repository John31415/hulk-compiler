use super::{Backend, BackendError, BackendResult};
use crate::semantic::{
    SemanticAnalyzer,
    hir::{TypedExpr, TypedExprKind},
    types::TypeId,
};
use inkwell::{AddressSpace, values::BasicValueEnum};

impl<'ctx> Backend<'ctx> {
    pub fn compile_if(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        if let TypedExprKind::If {
            condition,
            then_branch,
            else_branch,
        } = &expr.node
        {
            let cond_val = self.compile_expr(condition, sema)?.into_int_value();
            let start_bb = self.builder.get_insert_block().unwrap();
            let parent_fn = start_bb.get_parent().unwrap();
            let then_bb = self.llvm_context.append_basic_block(parent_fn, "if.then");
            let merge_bb = self.llvm_context.append_basic_block(parent_fn, "if.merge");
            let else_bb = if else_branch.is_some() {
                Some(self.llvm_context.append_basic_block(parent_fn, "if.else"))
            } else {
                None
            };
            let false_target = else_bb.unwrap_or(merge_bb);
            self.builder
                .build_conditional_branch(cond_val, then_bb, false_target)
                .map_err(|_| BackendError::InvalidExpression)?;
            self.builder.position_at_end(then_bb);
            let then_val = self.compile_expr(then_branch, sema)?;
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|_| BackendError::InvalidExpression)?;
            let source_then_bb = self.builder.get_insert_block().unwrap();
            let (else_val, source_else_bb) = if let Some(bb) = else_bb {
                self.builder.position_at_end(bb);
                let expr_inside_else = else_branch.as_ref().unwrap();
                let val = self.compile_expr(expr_inside_else, sema)?;
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| BackendError::InvalidExpression)?;
                (val, self.builder.get_insert_block().unwrap())
            } else {
                let llvm_return_ty = self.types.get_llvm_type(expr.ty);
                let default_val = match llvm_return_ty {
                    inkwell::types::BasicTypeEnum::PointerType(p) => {
                        BasicValueEnum::PointerValue(p.const_null())
                    }
                    inkwell::types::BasicTypeEnum::FloatType(f) => {
                        BasicValueEnum::FloatValue(f.const_zero())
                    }
                    inkwell::types::BasicTypeEnum::IntType(i) => {
                        BasicValueEnum::IntValue(i.const_zero())
                    }
                    _ => return Err(BackendError::InvalidExpression),
                };
                (default_val, start_bb)
            };
            self.builder.position_at_end(merge_bb);
            let llvm_return_ty = self.types.get_llvm_type(expr.ty);
            let phi_node = self
                .builder
                .build_phi(llvm_return_ty, "if_phi_result")
                .map_err(|_| BackendError::InvalidExpression)?;
            phi_node.add_incoming(&[(&then_val, source_then_bb), (&else_val, source_else_bb)]);
            return Ok(phi_node.as_basic_value());
        }
        Err(BackendError::InvalidExpression)
    }

    pub fn compile_while(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        let (condition, body) = match &expr.node {
            TypedExprKind::While { condition, body } => (condition, body),
            _ => return Err(BackendError::InvalidExpression),
        };
        let current_fn = self
            .builder
            .get_insert_block()
            .ok_or(BackendError::InvalidExpression)?
            .get_parent()
            .ok_or(BackendError::InvalidExpression)?;
        let cond_bb = self
            .llvm_context
            .append_basic_block(current_fn, "while.cond");
        let body_bb = self
            .llvm_context
            .append_basic_block(current_fn, "while.body");
        let after_bb = self
            .llvm_context
            .append_basic_block(current_fn, "while.after");
        let body_ty = self.types.get_llvm_type(body.ty);
        let result_alloca = self.create_entry_block_alloca("while.result", body_ty);
        let default_val = self.get_default_value(body.ty);
        self.builder
            .build_store(result_alloca, default_val)
            .map_err(|_| BackendError::InvalidExpression)?;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| BackendError::InvalidExpression)?;
        self.builder.position_at_end(cond_bb);
        let cond_val = self.compile_expr(condition, sema)?.into_int_value();
        self.builder
            .build_conditional_branch(cond_val, body_bb, after_bb)
            .map_err(|_| BackendError::InvalidExpression)?;
        self.builder.position_at_end(body_bb);
        self.push_scope();
        let body_val = self
            .compile_expr(body, sema)
            .map_err(|_| BackendError::InvalidExpression)?;
        self.pop_scope();
        self.builder
            .build_store(result_alloca, body_val)
            .map_err(|_| BackendError::InvalidExpression)?;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| BackendError::InvalidExpression)?;
        self.builder.position_at_end(after_bb);
        let final_val = self
            .builder
            .build_load(body_ty, result_alloca, "while.result")
            .map_err(|_| BackendError::InvalidExpression)?;
        Ok(final_val)
    }

    pub fn compile_for(
        &mut self,
        _expr: &TypedExpr,
        _sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        unimplemented!()
    }

    fn get_default_value(&self, ty: TypeId) -> BasicValueEnum<'ctx> {
        match ty {
            TypeId(3) => {
                let bool_ty = self.llvm_context.bool_type();
                BasicValueEnum::IntValue(bool_ty.const_int(0, false))
            }
            TypeId(1) => {
                let f64_ty = self.llvm_context.f64_type();
                BasicValueEnum::FloatValue(f64_ty.const_float(0.0))
            }
            _ => {
                let ptr_ty = self.llvm_context.ptr_type(AddressSpace::default());
                BasicValueEnum::PointerValue(ptr_ty.const_null())
            }
        }
    }
}
