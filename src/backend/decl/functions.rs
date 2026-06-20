use super::{Backend, BackendError, BackendResult};

use inkwell::AddressSpace;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};

use crate::{
    backend::functions::FunctionRegistry,
    semantic::{
        SemanticAnalyzer,
        hir::{TypedDecl, TypedDeclKind},
    },
};

impl<'ctx> Backend<'ctx> {
    pub fn declare_function(&mut self, decl: &TypedDecl) -> BackendResult<()> {
        if let TypedDeclKind::Function {
            name,
            params,
            return_type,
            ..
        } = &decl.node
        {
            let mut llvm_params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
            for param in params {
                let llvm_ty = self.types.get_llvm_type(param.node.type_id);
                llvm_params.push(llvm_ty.into());
            }
            let llvm_return = self.types.get_llvm_type(*return_type);
            let fn_type = llvm_return.fn_type(&llvm_params, false);
            let mangled_name = FunctionRegistry::mangle_global(name);
            let function_value = self.module.add_function(&mangled_name, fn_type, None);
            self.functions.insert_global(&mangled_name, function_value);
        }
        Ok(())
    }

    pub fn compile_function(
        &mut self,
        decl: &TypedDecl,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<()> {
        let TypedDeclKind::Function {
            name,
            params,
            return_type: _,
            body,
        } = &decl.node
        else {
            return Ok(());
        };
        let ptr_ty = self.llvm_context.ptr_type(AddressSpace::default());
        let bool_ty = self.types.bool_type;
        let number_ty = self.types.number_type;
        let string_ty = self.types.string_type;
        let llvm_type_of = |sem_id: crate::semantic::types::TypeId| -> BasicTypeEnum<'ctx> {
            let type_name = &sema.ctx.types.get(sem_id).name;
            match type_name.as_str() {
                "Boolean" => bool_ty.into(),
                "Number" => number_ty.into(),
                "String" => string_ty.into(),
                _ => ptr_ty.into(),
            }
        };
        let fn_name = FunctionRegistry::mangle_global(name);
        let function = self
            .functions
            .get_global(&fn_name)
            .ok_or_else(|| BackendError::UnknownFunction(fn_name.clone()))?;
        if function.get_first_basic_block().is_some() {
            return Ok(());
        }
        let entry = self.llvm_context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        let old_fn = self.current_function;
        let old_type = self.current_type;
        let old_method = self.current_method.clone();
        self.current_function = Some(function);
        self.current_type = None;
        self.current_method = None;
        self.push_scope();
        let result = (|| -> BackendResult<()> {
            for (i, param) in params.iter().enumerate() {
                let incoming = function
                    .get_nth_param(i as u32)
                    .ok_or(BackendError::InvalidExpression)?;
                let alloca = self
                    .builder
                    .build_alloca(llvm_type_of(param.node.type_id), &param.node.name)
                    .map_err(|_| BackendError::InvalidExpression)?;
                self.builder
                    .build_store(alloca, incoming)
                    .map_err(|_| BackendError::InvalidExpression)?;
                self.insert_local(param.node.name.clone(), alloca);
            }
            let body_val = self.compile_expr(body, sema)?;
            self.builder
                .build_return(Some(&body_val))
                .map_err(|_| BackendError::InvalidExpression)?;
            Ok(())
        })();
        self.pop_scope();
        self.current_function = old_fn;
        self.current_type = old_type;
        self.current_method = old_method;
        result
    }
}
