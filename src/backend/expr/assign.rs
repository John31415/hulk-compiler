use inkwell::values::{BasicValueEnum, PointerValue};

use crate::semantic::{
    SemanticAnalyzer,
    hir::{TypedExpr, TypedExprKind},
};

use super::{Backend, BackendError, BackendResult};

impl<'ctx> Backend<'ctx> {
    pub fn compile_assign(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<BasicValueEnum<'ctx>> {
        if let TypedExprKind::Assign { target, value } = &expr.node {
            let ptr = self.compile_lvalue(target, sema)?;
            let new_val = self.compile_expr(value, sema)?;
            self.builder
                .build_store(ptr, new_val)
                .map_err(|_| BackendError::InvalidExpression)?;
            Ok(new_val)
        } else {
            Err(BackendError::InvalidExpression)
        }
    }

    pub fn compile_lvalue(
        &mut self,
        expr: &TypedExpr,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<PointerValue<'ctx>> {
        match &expr.node {
            TypedExprKind::Variable(name) => self
                .lookup_local(name)
                .ok_or(BackendError::InvalidExpression),
            TypedExprKind::PropertyAccess { obj, property } => {
                let obj_val = self.compile_expr(obj, sema)?;
                let obj_ptr = obj_val.into_pointer_value();
                let obj_ty = obj.ty;
                let layout = self
                    .types
                    .get_layout(obj_ty)
                    .ok_or(BackendError::InvalidExpression)?;
                let mut index = None;
                for (i, field) in layout.field_names.iter().enumerate() {
                    if field == property {
                        index = Some(i + 1);
                        break;
                    }
                }
                let field_index = index.ok_or(BackendError::InvalidExpression)?;
                let field_ptr = self
                    .builder
                    .build_struct_gep(layout.struct_type, obj_ptr, field_index as u32, "field_ptr")
                    .map_err(|_| BackendError::InvalidExpression)?;
                Ok(field_ptr)
            }
            _ => Err(BackendError::InvalidExpression),
        }
    }
}
