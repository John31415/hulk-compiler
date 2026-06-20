use inkwell::{
    AddressSpace,
    types::{BasicMetadataTypeEnum, BasicType},
};

use crate::{
    backend::functions::FunctionRegistry,
    semantic::hir::{TypedTypeFeature, TypedTypeFeatureKind},
};

use super::{Backend, BackendResult};

use inkwell::types::BasicTypeEnum;

use crate::semantic::SemanticAnalyzer;

use super::BackendError;

impl<'ctx> Backend<'ctx> {
    pub fn declare_class_methods(
        &mut self,
        class_name: &str,
        features: &[TypedTypeFeature],
    ) -> BackendResult<()> {
        for feature in features {
            if let TypedTypeFeatureKind::Method {
                name,
                params,
                return_type,
                ..
            } = &feature.node
            {
                self.method_slots.register(name);

                let mut llvm_params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
                let self_ptr_type = self.llvm_context.ptr_type(AddressSpace::default());
                llvm_params.push(self_ptr_type.into());
                for param in params {
                    let llvm_ty = self.types.get_llvm_type(param.node.type_id);
                    llvm_params.push(llvm_ty.into());
                }
                let llvm_return = self.types.get_llvm_type(*return_type);
                let fn_type = llvm_return.fn_type(&llvm_params, false);
                let mangled_name = FunctionRegistry::mangle_method(class_name, name);
                let function_value = self.module.add_function(&mangled_name, fn_type, None);
                self.functions
                    .insert_method(class_name, name, function_value);
            }
        }
        Ok(())
    }

    pub fn compile_method(
        &mut self,
        type_name: &str,
        feature: &TypedTypeFeature,
        sema: &SemanticAnalyzer,
    ) -> BackendResult<()> {
        let TypedTypeFeatureKind::Method {
            name,
            params,
            return_type: _,
            body,
        } = &feature.node
        else {
            return Ok(());
        };
        let bool_ty = self.types.bool_type;
        let num_ty = self.types.number_type;
        let str_ty = self.types.string_type;
        let ptr_ty = self.llvm_context.ptr_type(AddressSpace::default());
        let types_ctx = &sema.ctx.types;
        let llvm_type_of = |sem_id| -> BasicTypeEnum<'ctx> {
            let name = &types_ctx.get(sem_id).name;
            match name.as_str() {
                "Boolean" => bool_ty.into(),
                "Number" => num_ty.into(),
                "String" => str_ty.into(),
                _ => ptr_ty.into(),
            }
        };
        let function = self.functions.get_method(type_name, name).ok_or_else(|| {
            BackendError::UnknownFunction(FunctionRegistry::mangle_method(type_name, name))
        })?;
        if function.get_first_basic_block().is_some() {
            return Ok(());
        }
        let entry = self.llvm_context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        let old_fn = self.current_function;
        let old_type = self.current_type;
        let old_method = self.current_method.clone();
        self.current_function = Some(function);
        self.current_type = sema.ctx.types.resolve(type_name);
        self.current_method = Some(name.clone());
        self.push_scope();
        let result = (|| -> BackendResult<()> {
            let self_param = function
                .get_nth_param(0)
                .ok_or(BackendError::InvalidExpression)?;
            let self_alloca = self
                .builder
                .build_alloca(ptr_ty, "self")
                .map_err(|_| BackendError::InvalidExpression)?;
            self.builder
                .build_store(self_alloca, self_param)
                .map_err(|_| BackendError::InvalidExpression)?;
            self.insert_local("self", self_alloca);
            for (i, param) in params.iter().enumerate() {
                let incoming = function
                    .get_nth_param((i + 1) as u32)
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
