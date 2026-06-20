use std::collections::HashMap;

use inkwell::{
    AddressSpace,
    context::Context,
    types::{BasicTypeEnum, FloatType, IntType, PointerType, StructType},
    values::GlobalValue,
};

use crate::semantic::types::TypeId;

pub struct TypeLayout<'ctx> {
    pub name: String,
    pub struct_type: StructType<'ctx>,
    pub parent: Option<TypeId>,
    pub field_names: Vec<String>,
    pub field_types: Vec<BasicTypeEnum<'ctx>>,
    pub vtable_struct_type: Option<StructType<'ctx>>,
    pub vtable_global: Option<GlobalValue<'ctx>>,
}

pub struct TypeRegistry<'ctx> {
    pub bool_type: IntType<'ctx>,
    pub number_type: FloatType<'ctx>,
    pub string_type: PointerType<'ctx>,
    pub name_to_id: HashMap<String, TypeId>,
    pub layouts: HashMap<TypeId, TypeLayout<'ctx>>,
}

impl<'ctx> TypeRegistry<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        let i1_ty = context.bool_type();
        let f64_ty = context.f64_type();
        let string_ty = context.ptr_type(AddressSpace::default());
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let mut registry = Self {
            bool_type: i1_ty,
            number_type: f64_ty,
            string_type: string_ty,
            name_to_id: HashMap::new(),
            layouts: HashMap::new(),
        };
        let object_id = TypeId(0);
        let object_struct = context.struct_type(&[ptr_ty.into()], false);
        registry.register_builtin("Object", object_id);
        registry.insert_layout(object_id, "Object", object_struct, None);
        registry
    }

    pub fn register_builtin(&mut self, name: impl Into<String>, id: TypeId) {
        self.name_to_id.insert(name.into(), id);
    }

    pub fn insert_layout(
        &mut self,
        type_id: TypeId,
        name: impl Into<String>,
        struct_type: StructType<'ctx>,
        parent: Option<TypeId>,
    ) {
        self.layouts.insert(
            type_id,
            TypeLayout {
                name: name.into(),
                struct_type,
                parent,
                field_names: Vec::new(),
                field_types: Vec::new(),
                vtable_struct_type: None,
                vtable_global: None,
            },
        );
    }

    pub fn get_layout(&self, type_id: TypeId) -> Option<&TypeLayout<'ctx>> {
        self.layouts.get(&type_id)
    }

    pub fn get_layout_mut(&mut self, type_id: TypeId) -> Option<&mut TypeLayout<'ctx>> {
        self.layouts.get_mut(&type_id)
    }

    pub fn get_llvm_type(&self, id: TypeId) -> BasicTypeEnum<'ctx> {
        if id == TypeId(1) {
            BasicTypeEnum::FloatType(self.number_type)
        } else if id == TypeId(3) {
            BasicTypeEnum::IntType(self.bool_type)
        } else {
            BasicTypeEnum::PointerType(self.string_type)
        }
    }

    pub fn get_field_info(
        &self,
        type_id: TypeId,
        property_name: &str,
    ) -> Option<(u32, BasicTypeEnum<'ctx>)> {
        let mut current_id = type_id;
        while let Some(layout) = self.get_layout(current_id) {
            if let Some(local_idx) = layout
                .field_names
                .iter()
                .position(|name| name == property_name)
            {
                let llvm_total_fields = layout.struct_type.count_fields();
                let local_fields_count = layout.field_names.len() as u32;
                let prefix_offset = llvm_total_fields.saturating_sub(local_fields_count);
                let physical_index = prefix_offset + (local_idx as u32);
                let field_type = layout.field_types[local_idx];
                return Some((physical_index, field_type));
            }
            current_id = layout.parent?;
        }
        None
    }

    pub fn is_subtype_of(&self, mut type_id: TypeId, target: TypeId) -> bool {
        loop {
            if type_id == target {
                return true;
            }
            match self.get_layout(type_id).and_then(|l| l.parent) {
                Some(parent_id) => type_id = parent_id,
                None => return false,
            }
        }
    }

    pub fn subtypes_of(&self, target: TypeId) -> Vec<TypeId> {
        self.layouts
            .keys()
            .copied()
            .filter(|&id| self.is_subtype_of(id, target))
            .collect()
    }
}
