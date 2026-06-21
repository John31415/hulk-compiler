use inkwell::values::{BasicValueEnum, PointerValue};

use crate::{
    backend::functions::FunctionRegistry,
    semantic::{
        SemanticAnalyzer,
        hir::{TypedExpr, TypedExprKind},
    },
};

use super::{Backend, BackendError, BackendResult};

impl<'ctx> Backend<'ctx> {
    pub fn compile_new(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        let TypedExprKind::New { name, args } = &expr.node else {
            return Err(BackendError::InvalidExpression);
        };
        let struct_type = self
            .types
            .get_layout(expr.ty)
            .ok_or(BackendError::InvalidExpression)?
            .struct_type;
        let ptr_val: PointerValue<'ctx> = self
            .builder
            .build_malloc(struct_type, &format!("new_{}", name))
            .map_err(|_| BackendError::InvalidExpression)?;
        let constructor_name = FunctionRegistry::mangle_constructor(name);
        if let Some(constructor_func) = self.module.get_function(&constructor_name) {
            let mut compiled_args = Vec::with_capacity(args.len() + 1);
            compiled_args.push(ptr_val.into());
            for arg in args {
                let arg_val = self.compile_expr(arg, sema)?;
                compiled_args.push(arg_val.into());
            }
            self.builder
                .build_call(constructor_func, &compiled_args, "call_constructor")
                .map_err(|_| BackendError::InvalidExpression)?;
        } else if !args.is_empty() {
            return Err(BackendError::InvalidExpression);
        }
        Ok(BasicValueEnum::PointerValue(ptr_val))
    }
}
