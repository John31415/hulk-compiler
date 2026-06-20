use crate::{
    backend::functions::FunctionRegistry,
    semantic::{
        SemanticAnalyzer,
        hir::{TypedDecl, TypedDeclKind, TypedTypeFeatureKind},
    },
};

use super::{Backend, BackendResult};

use inkwell::{AddressSpace, values::BasicMetadataValueEnum};

use super::BackendError;

use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};

impl<'ctx> Backend<'ctx> {
    pub fn declare_type(&mut self, decl: &TypedDecl) -> BackendResult<()> {
        if let TypedDeclKind::Type {
            name,
            parent,
            features,
            type_id,
            ..
        } = &decl.node
        {
            let struct_type = self.llvm_context.opaque_struct_type(name);
            let parent_id = parent.as_ref().map(|inherit| inherit.node.parent_type);
            self.types
                .insert_layout(*type_id, name, struct_type, parent_id);
            let mut field_data = Vec::new();
            for feature in features {
                if let TypedTypeFeatureKind::Attribute { name, type_id, .. } = &feature.node {
                    let llvm_ty = self.types.get_llvm_type(*type_id);
                    field_data.push((name.clone(), llvm_ty));
                }
            }
            if let Some(layout) = self.types.get_layout_mut(*type_id) {
                for (name, llvm_ty) in field_data {
                    layout.field_names.push(name);
                    layout.field_types.push(llvm_ty);
                }
            }
            // Registra el slot global de cada método ANTES de que se
            // construya ninguna vtable, para que el tamaño total de
            // slots (method_slots.total_slots()) sea correcto y estable
            // sin importar el orden en que se declaren los tipos.
            self.declare_class_methods(name, &features)?;
        }
        Ok(())
    }

    pub fn compile_type(&mut self, decl: &TypedDecl, sema: &SemanticAnalyzer) -> BackendResult<()> {
        let TypedDeclKind::Type {
            name,
            params,
            parent,
            features,
            type_id,
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
        let ctor_name = FunctionRegistry::mangle_constructor(name);
        let ctor_fn = if let Some(existing) = self.functions.get_constructor(&ctor_name) {
            if existing.get_first_basic_block().is_some() {
                return Ok(());
            }
            existing
        } else {
            let mut ctor_param_types: Vec<BasicMetadataTypeEnum<'ctx>> = vec![ptr_ty.into()];
            if let Some(param_list) = params {
                for p in param_list {
                    ctor_param_types.push(llvm_type_of(p.node.type_id).into());
                }
            }
            let ctor_fn_type = ptr_ty.fn_type(&ctor_param_types, false);
            let ctor_fn = self.module.add_function(&ctor_name, ctor_fn_type, None);
            self.functions.insert_constructor(&ctor_name, ctor_fn);
            ctor_fn
        };
        let (parent_type_id, parent_field_names, parent_field_types, parent_ctor_name) =
            if let Some(parent_info) = parent {
                let parent_layout = self
                    .types
                    .get_layout(parent_info.node.parent_type)
                    .ok_or_else(|| {
                        BackendError::UnknownType(parent_info.node.parent_type.0.to_string())
                    })?;
                (
                    Some(parent_info.node.parent_type),
                    parent_layout.field_names.clone(),
                    parent_layout.field_types.clone(),
                    Some(FunctionRegistry::mangle_constructor(&parent_layout.name)),
                )
            } else {
                (None, Vec::new(), Vec::new(), None)
            };
        let parent_field_count = parent_field_names.len();
        let struct_type = {
            let layout = self
                .types
                .get_layout_mut(*type_id)
                .ok_or_else(|| BackendError::UnknownType(name.clone()))?;
            layout.parent = parent_type_id;
            layout.field_names = parent_field_names.clone();
            layout.field_types = parent_field_types.clone();
            for feature in features {
                if let TypedTypeFeatureKind::Attribute {
                    name: attr_name,
                    type_id: attr_type_id,
                    ..
                } = &feature.node
                {
                    layout.field_names.push(attr_name.clone());
                    layout.field_types.push(llvm_type_of(*attr_type_id));
                }
            }
            // El campo 0 ahora es el puntero a la vtable del tipo
            // dinámico real (antes era un i32 con el tag crudo).
            let mut struct_fields: Vec<BasicTypeEnum<'ctx>> = vec![ptr_ty.into()];
            struct_fields.extend(layout.field_types.iter().copied());
            layout.struct_type.set_body(&struct_fields, false);
            layout.struct_type
        };

        // --- Construcción de la vtable de este tipo ---
        // Se hace aquí (no en declare_type) porque necesita que TODOS los
        // métodos de TODOS los tipos ya estén declarados (sus FunctionValue
        // deben existir, aunque su cuerpo todavía no se haya compilado, lo
        // cual es seguro: un puntero a función es válido en cuanto la
        // función fue añadida al módulo).
        self.build_vtable_global(name, *type_id)?;

        let entry = self.llvm_context.append_basic_block(ctor_fn, "entry");
        self.builder.position_at_end(entry);
        let old_fn = self.current_function;
        let old_type = self.current_type;
        let old_method = self.current_method.clone();
        self.current_function = Some(ctor_fn);
        self.current_type = Some(*type_id);
        self.current_method = None;
        self.push_scope();
        let result = (|| -> BackendResult<()> {
            let self_ptr = ctor_fn
                .get_nth_param(0)
                .ok_or(BackendError::InvalidExpression)?
                .into_pointer_value();
            self.insert_local("self", self_ptr);
            if let Some(param_list) = params {
                for (i, p) in param_list.iter().enumerate() {
                    let incoming = ctor_fn
                        .get_nth_param((i + 1) as u32)
                        .ok_or(BackendError::InvalidExpression)?;
                    let alloca = self
                        .builder
                        .build_alloca(llvm_type_of(p.node.type_id), &p.node.name)
                        .map_err(|_| BackendError::InvalidExpression)?;
                    self.builder
                        .build_store(alloca, incoming)
                        .map_err(|_| BackendError::InvalidExpression)?;
                    self.insert_local(p.node.name.clone(), alloca);
                }
            }
            if let (Some(_parent_type_id), Some(parent_ctor_name)) =
                (parent_type_id, parent_ctor_name.clone())
            {
                let parent_ctor = self
                    .functions
                    .get_constructor(&parent_ctor_name)
                    .ok_or_else(|| BackendError::UnknownFunction(parent_ctor_name.clone()))?;
                let mut call_args: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                call_args.push(self_ptr.into());
                if let Some(parent_args) = &parent.as_ref().and_then(|p| p.node.args.clone()) {
                    for arg in parent_args {
                        let arg_val = self.compile_expr(arg, sema)?;
                        call_args.push(arg_val.into());
                    }
                }
                self.builder
                    .build_call(parent_ctor, &call_args, "call_parent_ctor")
                    .map_err(|_| BackendError::InvalidExpression)?;
            }

            // Escribir el puntero a la vtable de ESTE tipo (no la del
            // padre) en el campo 0. Se hace después de llamar al
            // constructor padre para que el tag/vtable más derivado
            // siempre prevalezca sobre el que el padre pudo haber
            // escrito para sí mismo.
            let vtable_global = self
                .types
                .get_layout(*type_id)
                .and_then(|l| l.vtable_global)
                .ok_or(BackendError::InvalidExpression)?;
            let vtable_ptr = vtable_global.as_pointer_value();
            let vtable_slot_ptr = self
                .builder
                .build_struct_gep(struct_type, self_ptr, 0, "vtable_slot_ptr")
                .map_err(|_| BackendError::InvalidExpression)?;
            self.builder
                .build_store(vtable_slot_ptr, vtable_ptr)
                .map_err(|_| BackendError::InvalidExpression)?;

            let mut own_attr_index = 0usize;
            for feature in features {
                if let TypedTypeFeatureKind::Attribute {
                    name: attr_name,
                    type_id,
                    default,
                } = &feature.node
                {
                    let field_index = 1 + parent_field_count + own_attr_index;
                    own_attr_index += 1;
                    let field_ptr = self
                        .builder
                        .build_struct_gep(struct_type, self_ptr, field_index as u32, attr_name)
                        .map_err(|_| BackendError::InvalidExpression)?;
                    let init_val = if let Some(expr) = default {
                        self.compile_expr(expr, sema)?
                    } else {
                        llvm_type_of(*type_id).const_zero()
                    };
                    self.builder
                        .build_store(field_ptr, init_val)
                        .map_err(|_| BackendError::InvalidExpression)?;
                }
                if let TypedTypeFeatureKind::Method { .. } = &feature.node {
                    self.compile_method(name, feature, sema)?;
                    self.builder.position_at_end(entry);
                }
            }
            self.builder
                .build_return(Some(&self_ptr))
                .map_err(|_| BackendError::InvalidExpression)?;
            Ok(())
        })();
        self.pop_scope();
        self.current_function = old_fn;
        self.current_type = old_type;
        self.current_method = old_method;
        result
    }

    /// Construye la vtable global de `type_id` (nombre `name`), resolviendo
    /// para cada slot de método global cuál implementación usar:
    ///
    /// 1. Si este tipo (o algún ancestro) declara el método, usa la
    ///    implementación más derivada (la del primer tipo, subiendo desde
    ///    `type_id`, que lo define).
    /// 2. Si ningún ancestro lo declara, el slot apunta a la función trap
    ///    `hulk_unreachable_method` (no debería invocarse nunca si el
    ///    análisis semántico es correcto).
    ///
    /// La vtable se representa como `{ i32 tag, [N-1 x ptr] methods }`:
    /// el tag (usado por `is`/`as`) va en un campo aparte, y los punteros
    /// a función ocupan el arreglo, indexados por `method_slots` menos 1
    /// (el slot 0 global, reservado al tag, no tiene entrada en el array).
    fn build_vtable_global(
        &mut self,
        name: &str,
        type_id: crate::semantic::types::TypeId,
    ) -> BackendResult<()> {
        let ptr_ty = self.llvm_context.ptr_type(AddressSpace::default());
        let i32_ty = self.llvm_context.i32_type();
        let total_slots = self.method_slots.total_slots();
        let method_count = total_slots - 1; // sin contar el slot 0 (tag)

        let unreachable_fn = self
            .runtime
            .unreachable_method
            .ok_or(BackendError::InvalidExpression)?;

        let mut method_ptrs: Vec<inkwell::values::PointerValue<'ctx>> =
            Vec::with_capacity(method_count as usize);

        for slot_index in 1..total_slots {
            let method_name = self
                .method_slots
                .name_for_slot(slot_index)
                .ok_or(BackendError::InvalidExpression)?
                .to_string();

            // Busca, subiendo por la cadena de padres desde `type_id`, el
            // primer tipo que define `method_name`. Se hace manualmente
            // para no tomar prestado `self.types` y `self.functions` al
            // mismo tiempo dentro de un closure.
            let mut search_id = type_id;
            let fn_ptr = loop {
                let owner_name = self
                    .types
                    .get_layout(search_id)
                    .ok_or(BackendError::InvalidExpression)?
                    .name
                    .clone();
                if let Some(func) = self.functions.get_method(&owner_name, &method_name) {
                    break func.as_global_value().as_pointer_value();
                }
                let parent = self
                    .types
                    .get_layout(search_id)
                    .ok_or(BackendError::InvalidExpression)?
                    .parent;
                match parent {
                    Some(parent_id) => search_id = parent_id,
                    None => break unreachable_fn.as_global_value().as_pointer_value(),
                }
            };
            method_ptrs.push(fn_ptr);
        }

        let methods_array_type = ptr_ty.array_type(method_count);
        let methods_array_const = ptr_ty.const_array(&method_ptrs);

        let vtable_struct_type = self
            .llvm_context
            .struct_type(&[i32_ty.into(), methods_array_type.into()], false);
        let tag_const = i32_ty.const_int(type_id.0 as u64, false);
        let vtable_const =
            vtable_struct_type.const_named_struct(&[tag_const.into(), methods_array_const.into()]);

        let global_name = format!("vtable_{}", name);
        let global = self
            .module
            .add_global(vtable_struct_type, None, &global_name);
        global.set_initializer(&vtable_const);
        global.set_constant(true);

        if let Some(layout) = self.types.get_layout_mut(type_id) {
            layout.vtable_struct_type = Some(vtable_struct_type);
            layout.vtable_global = Some(global);
        }

        Ok(())
    }
}
