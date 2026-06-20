use inkwell::values::FunctionValue;
use std::collections::HashMap;

pub struct FunctionRegistry<'ctx> {
    pub globals: HashMap<String, FunctionValue<'ctx>>,
    pub methods: HashMap<String, FunctionValue<'ctx>>,
    pub constructors: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> FunctionRegistry<'ctx> {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            methods: HashMap::new(),
            constructors: HashMap::new(),
        }
    }

    pub fn mangle_global(name: &str) -> String {
        format!("hulk_fn_{name}")
    }

    pub fn mangle_method(type_name: &str, method_name: &str) -> String {
        format!("hulk_method_{type_name}_{method_name}")
    }

    pub fn mangle_constructor(type_name: &str) -> String {
        format!("hulk_ctor_{type_name}")
    }

    pub fn insert_global(&mut self, name: &str, fun: FunctionValue<'ctx>) {
        self.globals.insert(name.to_string(), fun);
    }

    pub fn insert_method(&mut self, type_name: &str, method_name: &str, fun: FunctionValue<'ctx>) {
        let mangled = Self::mangle_method(type_name, method_name);
        self.methods.insert(mangled, fun);
    }

    pub fn insert_constructor(&mut self, type_name: &str, fun: FunctionValue<'ctx>) {
        let mangled = Self::mangle_constructor(type_name);
        self.constructors.insert(mangled, fun);
    }

    pub fn get_global(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.globals.get(name).copied()
    }

    pub fn get_method(&self, type_name: &str, method_name: &str) -> Option<FunctionValue<'ctx>> {
        let mangled = Self::mangle_method(type_name, method_name);
        self.methods.get(&mangled).copied()
    }

    pub fn get_constructor(&self, type_name: &str) -> Option<FunctionValue<'ctx>> {
        let mangled = Self::mangle_constructor(type_name);
        self.constructors.get(&mangled).copied()
    }
}
