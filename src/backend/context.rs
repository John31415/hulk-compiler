use std::collections::HashMap;

use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::BasicTypeEnum,
    values::{FunctionValue, PointerValue},
};

use crate::semantic::types::TypeId;

use super::{
    functions::FunctionRegistry, method_slots::MethodSlotRegistry,
    runtime::RuntimeRegistry, types::TypeRegistry,
};

pub struct Backend<'ctx> {
    pub llvm_context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,

    pub types: TypeRegistry<'ctx>,
    pub functions: FunctionRegistry<'ctx>,
    pub runtime: RuntimeRegistry<'ctx>,
    pub method_slots: MethodSlotRegistry,

    pub current_function: Option<FunctionValue<'ctx>>,
    pub current_type: Option<TypeId>,
    pub current_method: Option<String>,

    pub scopes: Vec<HashMap<String, PointerValue<'ctx>>>,
}

impl<'ctx> Backend<'ctx> {
    pub fn new(llvm_context: &'ctx Context, module_name: &str) -> Self {
        let module = llvm_context.create_module(module_name);
        let builder = llvm_context.create_builder();
        Self::declare_constants(&module, llvm_context);
        let mut runtime = RuntimeRegistry::new();
        runtime.init(llvm_context, &module);

        Self {
            llvm_context,
            module,
            builder,
            types: TypeRegistry::new(llvm_context),
            functions: FunctionRegistry::new(),
            runtime,
            method_slots: MethodSlotRegistry::new(),

            current_function: None,
            current_type: None,
            current_method: None,
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn insert_local(&mut self, name: impl Into<String>, ptr: PointerValue<'ctx>) {
        self.scopes.last_mut().unwrap().insert(name.into(), ptr);
    }

    pub fn lookup_local(&self, name: &str) -> Option<PointerValue<'ctx>> {
        for scope in self.scopes.iter().rev() {
            if let Some(ptr) = scope.get(name) {
                return Some(*ptr);
            }
        }
        None
    }

    pub fn create_entry_block_alloca(
        &self,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
    ) -> PointerValue<'ctx> {
        let current_fn = self.current_function.expect("No active function");
        let entry_bb = current_fn
            .get_first_basic_block()
            .expect("The function does not have entry block");
        let temp_builder = self.llvm_context.create_builder();
        if let Some(first_instruction) = entry_bb.get_first_instruction() {
            temp_builder.position_before(&first_instruction);
        } else {
            temp_builder.position_at_end(entry_bb);
        }
        temp_builder.build_alloca(ty, name).unwrap()
    }

    fn declare_constants(module: &Module<'ctx>, context: &'ctx Context) {
        let pi_value = context.f64_type().const_float(3.14159265358979323846);
        let pi_global = module.add_global(context.f64_type(), None, "PI");
        pi_global.set_initializer(&pi_value);
        pi_global.set_constant(true);
        let e_value = context.f64_type().const_float(2.71828182845904523536);
        let e_global = module.add_global(context.f64_type(), None, "E");
        e_global.set_initializer(&e_value);
        e_global.set_constant(true);
    }
}
