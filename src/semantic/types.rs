use super::symbols::Symbol;
use crate::semantic::symbols::SymbolType;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub usize);

#[derive(Debug, Clone)]
pub struct ConstructorParam {
    pub name: String,
    pub ty: Option<TypeId>,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub parent: Option<TypeId>,
    pub constructor_params: Vec<ConstructorParam>,
    pub attributes: HashMap<String, Symbol>,
    pub methods: HashMap<String, Symbol>,
}

pub struct TypeTable {
    by_name: HashMap<String, TypeId>,
    pub infos: Vec<TypeInfo>,
}

impl TypeTable {
    pub fn new() -> Self {
        let mut table = Self {
            by_name: HashMap::new(),
            infos: Vec::new(),
        };
        table.insert("Object".into(), None);
        table.insert("Number".into(), Some(TypeId(0)));
        table.insert("String".into(), Some(TypeId(0)));
        table.insert("Boolean".into(), Some(TypeId(0)));
        table
    }

    pub fn resolve(&self, name: &str) -> Option<TypeId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: TypeId) -> &TypeInfo {
        &self.infos[id.0]
    }

    pub fn insert(&mut self, name: String, parent: Option<TypeId>) -> Option<TypeId> {
        if self.by_name.contains_key(&name) {
            return None;
        }
        let id = TypeId(self.infos.len());
        self.infos.push(TypeInfo {
            name: name.clone(),
            parent,
            constructor_params: Vec::new(),
            attributes: HashMap::new(),
            methods: HashMap::new(),
        });
        self.by_name.insert(name, id);
        Some(id)
    }

    pub fn is_subtype_of(&self, left: TypeId, right: TypeId) -> bool {
        if left == right {
            return true;
        }
        let mut current = Some(left);
        while let Some(id) = current {
            if id == right {
                return true;
            }
            current = self.get(id).parent;
        }
        false
    }

    pub fn insert_attribute(&mut self, id: TypeId, attr: Symbol) -> bool {
        let info = &mut self.infos[id.0];
        if info.attributes.contains_key(&attr.name) {
            return false;
        }
        info.attributes.insert(attr.name.clone(), attr);
        true
    }

    pub fn insert_method(&mut self, id: TypeId, method: Symbol) -> bool {
        let info = &mut self.infos[id.0];
        if info.methods.contains_key(&method.name) {
            return false;
        }
        info.methods.insert(method.name.clone(), method);
        true
    }

    pub fn get_method(&self, type_id: TypeId, name: &str) -> Option<&Symbol> {
        let mut current = Some(type_id);
        while let Some(id) = current {
            let info = self.get(id);
            if let Some(method) = info.methods.get(name) {
                return Some(method);
            }
            current = info.parent;
        }
        None
    }

    pub fn get_constructor_params(&self, type_id: TypeId) -> &[ConstructorParam] {
        &self.infos[type_id.0].constructor_params
    }

    pub fn get_parent(&self, type_id: TypeId) -> Option<TypeId> {
        self.infos[type_id.0].parent
    }

    pub fn get_method_return_type(&self, type_id: TypeId, method_name: &str) -> Option<TypeId> {
        let info = &self.infos[type_id.0];
        if let Some(method_symbol) = info.methods.get(method_name) {
            if let SymbolType::Variable(return_type_id) = method_symbol.ty {
                return Some(return_type_id);
            }
        }
        None
    }

    pub fn find_lca(&self, a: TypeId, b: TypeId) -> TypeId {
        let mut ancestors_a = HashSet::new();
        let mut current = Some(a);
        while let Some(id) = current {
            ancestors_a.insert(id);
            current = self.get_parent(id);
        }
        let mut current = Some(b);
        while let Some(id) = current {
            if ancestors_a.contains(&id) {
                return id;
            }
            current = self.get_parent(id);
        }
        self.resolve("Object").unwrap()
    }

    pub fn lookup_attribute(&self, type_id: TypeId, name: &str) -> Option<TypeId> {
        let mut current = Some(type_id);
        while let Some(id) = current {
            let info = &self.infos[id.0];
            if let Some(symbol) = info.attributes.get(name) {
                if let SymbolType::Variable(ty_id) = symbol.ty {
                    return Some(ty_id);
                }
            }
            current = info.parent;
        }
        None
    }

    pub fn lookup_method(&self, type_id: TypeId, name: &str) -> Option<(Vec<TypeId>, TypeId)> {
        let mut current = Some(type_id);
        while let Some(id) = current {
            let info = &self.infos[id.0];
            if let Some(symbol) = info.methods.get(name) {
                if let SymbolType::Function { params, ret } = &symbol.ty {
                    return Some((params.clone(), *ret));
                }
            }
            current = info.parent;
        }
        None
    }

    pub fn set_parent(&mut self, type_id: TypeId, parent_id: Option<TypeId>) {
        if let Some(type_info) = self.infos.get_mut(type_id.0) {
            type_info.parent = parent_id;
        }
    }

    pub fn set_constructor_params(&mut self, type_id: TypeId, params: Vec<ConstructorParam>) {
        if let Some(type_info) = self.infos.get_mut(type_id.0) {
            type_info.constructor_params = params;
        }
    }
}
