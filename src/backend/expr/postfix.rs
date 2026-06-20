use super::{Backend, BackendError, BackendResult};
use crate::semantic::{
    SemanticAnalyzer,
    hir::{TypedExpr, TypedExprKind},
};
use inkwell::{
    AddressSpace,
    types::{BasicMetadataTypeEnum, BasicType},
    values::{BasicMetadataValueEnum, BasicValueEnum},
};

impl<'ctx> Backend<'ctx> {
    pub fn compile_property_access(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        let (obj_expr, property_name) = match &expr.node {
            TypedExprKind::PropertyAccess { obj, property } => (obj.as_ref(), property),
            _ => return Err(BackendError::InvalidExpression),
        };
        let obj_val = self.compile_expr(obj_expr, sema)?;
        let obj_ptr = obj_val.into_pointer_value();
        let struct_type = self
            .types
            .get_layout(obj_expr.ty)
            .ok_or(BackendError::InvalidExpression)?
            .struct_type;
        let (field_index, field_llvm_type) = self
            .types
            .get_field_info(obj_expr.ty, property_name)
            .ok_or(BackendError::InvalidExpression)?;
        let field_ptr = self
            .builder
            .build_struct_gep(struct_type, obj_ptr, field_index, "get_property_ptr")
            .map_err(|_| BackendError::InvalidExpression)?;
        let loaded_value = self
            .builder
            .build_load(field_llvm_type, field_ptr, "load_property")
            .map_err(|_| BackendError::InvalidExpression)?;
        Ok(loaded_value)
    }

    pub fn compile_method_call(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        let (obj_expr, method_name, args) = match &expr.node {
            TypedExprKind::MethodCall { obj, method, args } => (obj.as_ref(), method, args),
            _ => return Err(BackendError::InvalidExpression),
        };
        let obj_val = self.compile_expr(obj_expr, sema)?;
        let obj_ptr = obj_val.into_pointer_value();
        let mut compiled_args: Vec<BasicMetadataValueEnum<'ctx>> =
            Vec::with_capacity(args.len() + 1);
        compiled_args.push(obj_ptr.into());
        for arg_expr in args {
            let arg_val = self.compile_expr(arg_expr, sema)?;
            compiled_args.push(arg_val.into());
        }
        let mut search_id = obj_expr.ty;
        let reference_fn = loop {
            let layout = self
                .types
                .get_layout(search_id)
                .ok_or(BackendError::InvalidExpression)?;
            if let Some(func) = self.functions.get_method(&layout.name, method_name) {
                break func;
            }
            match layout.parent {
                Some(parent_id) => search_id = parent_id,
                None => return Err(BackendError::InvalidExpression),
            }
        };
        let fn_type = reference_fn.get_type();
        if fn_type.get_return_type().is_none() {
            return Err(BackendError::InvalidExpression);
        }
        let slot = self
            .method_slots
            .get(method_name)
            .ok_or(BackendError::InvalidExpression)?;
        let vtable_struct_type = self
            .types
            .get_layout(obj_expr.ty)
            .and_then(|l| l.vtable_struct_type)
            .or_else(|| {
                self.types
                    .layouts
                    .values()
                    .find_map(|l| l.vtable_struct_type)
            })
            .ok_or(BackendError::InvalidExpression)?;
        let ptr_ty = self.llvm_context.ptr_type(AddressSpace::default());
        let object_struct_type = self
            .types
            .get_layout(obj_expr.ty)
            .ok_or(BackendError::InvalidExpression)?
            .struct_type;
        let vtable_field_ptr = self
            .builder
            .build_struct_gep(object_struct_type, obj_ptr, 0, "vtable_field_ptr")
            .map_err(|_| BackendError::InvalidExpression)?;
        let vtable_ptr = self
            .builder
            .build_load(ptr_ty, vtable_field_ptr, "load_vtable_ptr")
            .map_err(|_| BackendError::InvalidExpression)?
            .into_pointer_value();
        let i32_ty = self.llvm_context.i32_type();
        let zero = i32_ty.const_int(0, false);
        let methods_field_idx = i32_ty.const_int(1, false);
        let method_idx = i32_ty.const_int((slot - 1) as u64, false);
        let method_slot_ptr = unsafe {
            self.builder
                .build_in_bounds_gep(
                    vtable_struct_type,
                    vtable_ptr,
                    &[zero, methods_field_idx, method_idx],
                    "method_slot_ptr",
                )
                .map_err(|_| BackendError::InvalidExpression)?
        };
        let raw_fn_ptr = self
            .builder
            .build_load(ptr_ty, method_slot_ptr, "load_method_ptr")
            .map_err(|_| BackendError::InvalidExpression)?
            .into_pointer_value();
        let param_types: Vec<BasicMetadataTypeEnum<'ctx>> = fn_type
            .get_param_types()
            .into_iter()
            .map(|t| t.into())
            .collect();
        let typed_fn_type = fn_type
            .get_return_type()
            .unwrap()
            .fn_type(&param_types, false);
        let call_site = self
            .builder
            .build_indirect_call(typed_fn_type, raw_fn_ptr, &compiled_args, "call_method")
            .map_err(|_| BackendError::InvalidExpression)?;
        Ok(call_site.try_as_basic_value().unwrap_basic())
    }
}
