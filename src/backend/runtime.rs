use std::collections::HashMap;

use inkwell::{
    context::Context,
    module::Module,
    types::FunctionType,
    values::{BasicValueEnum, FunctionValue},
    AddressSpace,
};

use crate::backend::functions::FunctionRegistry;

pub struct RuntimeRegistry<'ctx> {
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub constants: HashMap<String, BasicValueEnum<'ctx>>,
    pub unreachable_method: Option<FunctionValue<'ctx>>,
}

impl<'ctx> RuntimeRegistry<'ctx> {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            constants: HashMap::new(),
            unreachable_method: None,
        }
    }

    pub fn init(&mut self, ctx: &'ctx Context, module: &Module<'ctx>) {
        self.register_functions(ctx, module);
        self.register_constants(ctx);
        self.register_unreachable_method(ctx, module);
    }

    fn register_functions(&mut self, ctx: &'ctx Context, module: &Module<'ctx>) {
        let number = ctx.f64_type();
        let string = ctx.ptr_type(AddressSpace::default());
        let nullary_fn_type = number.fn_type(&[], false);
        let unary_fn_type = number.fn_type(&[number.into()], false);
        let binary_fn_type = number.fn_type(&[number.into(), number.into()], false);
        let print_fn_type = string.fn_type(&[string.into()], false);
        let print_number_fn_type = number.fn_type(&[number.into()], false);
        self.insert_function(&module, "sin", unary_fn_type);
        self.insert_function(&module, "cos", unary_fn_type);
        self.insert_function(&module, "exp", unary_fn_type);
        self.insert_function(&module, "sqrt", unary_fn_type);
        self.insert_function(&module, "log", binary_fn_type);
        self.insert_function(&module, "rand", nullary_fn_type);
        self.insert_function(&module, "print", print_fn_type);
        self.insert_function(&module, "print_number", print_number_fn_type);
    }

    fn register_constants(&mut self, ctx: &'ctx Context) {
        let number = ctx.f64_type();
        let pi = number.const_float(std::f64::consts::PI);
        let e = number.const_float(std::f64::consts::E);
        self.constants
            .insert("PI".to_string(), BasicValueEnum::FloatValue(pi));
        self.constants
            .insert("E".to_string(), BasicValueEnum::FloatValue(e));
    }

    fn register_unreachable_method(&mut self, ctx: &'ctx Context, module: &Module<'ctx>) {
        let void_fn_type = ctx.void_type().fn_type(&[], false);
        let function = module.add_function("hulk_unreachable_method", void_fn_type, None);
        self.unreachable_method = Some(function);
    }

    pub fn insert_function(
        &mut self,
        module: &Module<'ctx>,
        name: &str,
        fn_type: FunctionType<'ctx>,
    ) -> FunctionValue<'ctx> {
        let mangled_name = FunctionRegistry::mangle_global(name);
        let function = module.add_function(&mangled_name, fn_type, None);
        self.functions.insert(mangled_name, function);
        function
    }
}
