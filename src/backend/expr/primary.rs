use inkwell::values::BasicValueEnum;

use crate::semantic::hir::{TypedExpr, TypedExprKind};

use super::{Backend, BackendError, BackendResult};

use crate::ast::LiteralKind;

impl<'ctx> Backend<'ctx> {
    pub fn compile_literal(&mut self, expr: &TypedExpr) -> BackendResult<BasicValueEnum<'ctx>> {
        if let TypedExprKind::Literal(lit) = &expr.node {
            match lit {
                LiteralKind::Number(val) => {
                    let llvm_val = self.types.number_type.const_float(*val);
                    return Ok(BasicValueEnum::FloatValue(llvm_val));
                }
                LiteralKind::Bool(val) => {
                    let llvm_val = self.types.bool_type.const_int(*val as u64, false);
                    return Ok(BasicValueEnum::IntValue(llvm_val));
                }
                LiteralKind::String(val) => {
                    let global_value = self
                        .builder
                        .build_global_string_ptr(val, "str_lit")
                        .map_err(|_| BackendError::InvalidExpression)?;
                    let global_str_ptr = global_value.as_pointer_value();
                    return Ok(BasicValueEnum::PointerValue(global_str_ptr));
                }
            }
        }
        Err(BackendError::InvalidExpression)
    }

    pub fn compile_variable(&mut self, expr: &TypedExpr) -> BackendResult<BasicValueEnum<'ctx>> {
        if let TypedExprKind::Variable(name) = &expr.node {
            if let Some(ptr) = self.lookup_local(name) {
                let llvm_ty = self.types.get_llvm_type(expr.ty);
                let loaded_val = self
                    .builder
                    .build_load(llvm_ty, ptr, &format!("load.{name}"))
                    .map_err(|_| BackendError::InvalidExpression)?;
                return Ok(loaded_val);
            }
            if let Some(global) = self.module.get_global(name) {
                let llvm_ty = self.types.get_llvm_type(expr.ty);
                let loaded_val = self
                    .builder
                    .build_load(llvm_ty, global.as_pointer_value(), &format!("load.{name}"))
                    .map_err(|_| BackendError::InvalidExpression)?;
                return Ok(loaded_val);
            }
            return Err(BackendError::UndefinedVariable);
        }
        Err(BackendError::InvalidExpression)
    }
}
