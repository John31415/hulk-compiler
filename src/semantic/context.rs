use crate::semantic::symbols::SymbolType;

use super::{
    symbols::Symbol,
    types::{TypeId, TypeTable},
};
use std::collections::HashMap;

pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
}

impl Scope {
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.symbols.get_mut(name)
    }
}

pub struct SemanticContext {
    pub scopes: Vec<Scope>,
    pub types: TypeTable,
    pub current_method: Option<String>,
    pub current_type: Option<TypeId>,
    pub current_function_return: Option<TypeId>,
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
}
