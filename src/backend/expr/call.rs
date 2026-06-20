use super::{Backend, BackendError, BackendResult};
use crate::{
    backend::functions::FunctionRegistry,
    semantic::{
        SemanticAnalyzer,
        hir::{TypedExpr, TypedExprKind},
    },
};
use inkwell::{AddressSpace, values::BasicMetadataValueEnum, values::BasicValueEnum};

impl<'ctx> Backend<'ctx> {
    pub fn compile_call(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        if let TypedExprKind::Call { name, args } = &expr.node {
            if name == "base" {
                return self.compile_base_call(args, sema);
            }

            let mangled_name = FunctionRegistry::mangle_global(name);
            let function = self
                .module
                .get_function(&mangled_name)
                .ok_or(BackendError::InvalidExpression)?;
            let mut compiled_args: Vec<BasicMetadataValueEnum<'ctx>> =
                Vec::with_capacity(args.len());
            for arg_expr in args {
                let arg_val = self.compile_expr(arg_expr, sema)?;
                compiled_args.push(arg_val.into());
            }
            let call_site = self
                .builder
                .build_call(function, &compiled_args, "call_global")
                .map_err(|_| BackendError::InvalidExpression)?;
            return Ok(call_site.try_as_basic_value().unwrap_basic());
        }
        Err(BackendError::InvalidExpression)
    }

    fn compile_base_call(
        &mut self,
        args: &[TypedExpr],
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        let current_type = self.current_type.ok_or(BackendError::InvalidExpression)?;
        let method_name = self
            .current_method
            .clone()
            .ok_or(BackendError::InvalidExpression)?;

        let layout = self
            .types
            .get_layout(current_type)
            .ok_or(BackendError::InvalidExpression)?;
        let parent_id = layout.parent.ok_or(BackendError::InvalidExpression)?;

        let mut search_id = parent_id;
        let function = loop {
            let l = self
                .types
                .get_layout(search_id)
                .ok_or(BackendError::InvalidExpression)?;
            if let Some(func) = self.functions.get_method(&l.name, &method_name) {
                break func;
            }
            search_id = l.parent.ok_or(BackendError::InvalidExpression)?;
        };

        let self_ptr = self
            .lookup_local("self")
            .ok_or(BackendError::InvalidExpression)?;
        let ptr_ty = self.llvm_context.ptr_type(AddressSpace::default());
        let self_val = self
            .builder
            .build_load(ptr_ty, self_ptr, "load_self")
            .map_err(|_| BackendError::InvalidExpression)?;

        let mut compiled_args: Vec<BasicMetadataValueEnum<'ctx>> =
            Vec::with_capacity(args.len() + 1);
        compiled_args.push(self_val.into());
        for arg_expr in args {
            let arg_val = self.compile_expr(arg_expr, sema)?;
            compiled_args.push(arg_val.into());
        }

        let call_site = self
            .builder
            .build_call(function, &compiled_args, "call_base")
            .map_err(|_| BackendError::InvalidExpression)?;
        Ok(call_site.try_as_basic_value().unwrap_basic())
    }
}
