use crate::ast::Decl;
use crate::semantic::hir::TypedDecl;
use crate::semantic::symbols::SymbolType;

use super::{
    symbols::Symbol,
    types::{TypeId, TypeTable},
};
use std::collections::{HashMap, HashSet};

pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
}

impl Scope {
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.symbols.get_mut(name)
    }
}

pub type GenericInstanceKey = (String, Vec<TypeId>);

pub struct SemanticContext {
    pub scopes: Vec<Scope>,
    pub types: TypeTable,
    pub current_method: Option<String>,
    pub current_type: Option<TypeId>,
    pub current_function_return: Option<TypeId>,
    pub generic_decls: HashMap<String, Decl>,
    pub generic_instances: HashMap<GenericInstanceKey, TypedDecl>,
    pub instantiation_order: Vec<GenericInstanceKey>,
    pub in_progress_instances: HashSet<GenericInstanceKey>,
}

impl SemanticContext {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                symbols: HashMap::new(),
            }],
            types: TypeTable::new(),
            current_method: None,
            current_type: None,
            current_function_return: None,
            generic_decls: HashMap::new(),
            generic_instances: HashMap::new(),
            instantiation_order: Vec::new(),
            in_progress_instances: HashSet::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope {
            symbols: HashMap::new(),
        });
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(&mut self, symbol: Symbol) -> bool {
        let scope = self.scopes.last_mut().unwrap();
        if scope.symbols.contains_key(&symbol.name) {
            return false;
        }
        scope.symbols.insert(symbol.name.clone(), symbol);
        true
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.symbols.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    pub fn update_symbol_type(&mut self, name: &str, new_ty: SymbolType) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.ty = new_ty;
                return true;
            }
        }
        false
    }
    pub fn register_generic_decl(&mut self, name: String, decl: Decl) {
        self.generic_decls.insert(name, decl);
    }
    pub fn get_instance(&self, key: &GenericInstanceKey) -> Option<&TypedDecl> {
        self.generic_instances.get(key)
    }
    pub fn insert_instance(&mut self, key: GenericInstanceKey, typed_decl: TypedDecl) {
        if !self.generic_instances.contains_key(&key) {
            self.instantiation_order.push(key.clone());
        }
        self.generic_instances.insert(key, typed_decl);
    }
    pub fn mark_in_progress(&mut self, key: GenericInstanceKey) {
        self.in_progress_instances.insert(key);
    }
    pub fn unmark_in_progress(&mut self, key: &GenericInstanceKey) {
        self.in_progress_instances.remove(key);
    }
    pub fn is_in_progress(&self, key: &GenericInstanceKey) -> bool {
        self.in_progress_instances.contains(key)
    }
    pub fn mangle_instance_name(&self, base_name: &str, concrete_types: &[TypeId]) -> String {
        if concrete_types.is_empty() {
            return base_name.to_string();
        }
        let mut mangled = base_name.to_string();
        for ty in concrete_types {
            mangled.push('$');
            mangled.push_str(&self.types.get(*ty).name);
        }
        mangled
    }
}