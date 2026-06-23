use super::symbols::Symbol;
use crate::semantic::context::SemanticContext;
use crate::semantic::symbols::SymbolType;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct TypeId(pub usize);

#[derive(Debug, Clone)]
pub struct ConstructorParam {
    pub name: String,
    pub ty: Option<TypeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Class,
    Protocol { parents: Vec<TypeId> },
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub kind: TypeKind,
    pub name: String,
    pub parent: Option<TypeId>,
    pub declared_constructor_params: Option<Vec<ConstructorParam>>,
    pub constructor_params: Vec<ConstructorParam>,
    pub attributes: HashMap<String, Symbol>,
    pub methods: HashMap<String, Symbol>,
    pub is_generic_template: bool,
}

impl TypeInfo {
    pub fn is_protocol(&self) -> bool {
        matches!(&self.kind, TypeKind::Protocol { .. })
    }
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
            kind: TypeKind::Class,
            name: name.clone(),
            parent,
            declared_constructor_params: None,
            constructor_params: Vec::new(),
            attributes: HashMap::new(),
            methods: HashMap::new(),
            is_generic_template: false,
        });
        self.by_name.insert(name, id);
        Some(id)
    }

    pub fn insert_instantiation(&mut self, mangled_name: String, parent: Option<TypeId>) -> TypeId {
        let id = TypeId(self.infos.len());
        self.infos.push(TypeInfo {
            kind: TypeKind::Class,
            name: mangled_name.clone(),
            parent,
            declared_constructor_params: None,
            constructor_params: Vec::new(),
            attributes: HashMap::new(),
            methods: HashMap::new(),
            is_generic_template: false,
        });
        self.by_name.insert(mangled_name, id);
        id
    }

    pub fn is_subtype_of(&self, ctx: &SemanticContext, left: TypeId, right: TypeId) -> bool {
        if left == right {
            return true;
        }
        let is_basic_type = |id: TypeId| id.0 <= 3;
        let left_kind = ctx.types.infos[left.0].kind.clone();
        let right_kind = ctx.types.infos[right.0].kind.clone();
        if !is_basic_type(left) && matches!(right_kind, TypeKind::Protocol { .. }) {
            if matches!(left_kind, TypeKind::Protocol { .. }) {
                let mut queue = VecDeque::new();
                let mut used = HashMap::new();
                queue.push_back(left);
                used.insert(left, true);
                while let Some(id) = queue.pop_front() {
                    if id == right {
                        return true;
                    }
                    if let TypeKind::Protocol { parents } = &ctx.types.infos[id.0].kind {
                        for parent in parents {
                            if !used.contains_key(parent) {
                                used.insert(*parent, true);
                                queue.push_back(*parent);
                            }
                        }
                    }
                }
            } else {
                let mut expected_methods: Vec<Symbol> = Vec::new();
                for (_, m) in &ctx.types.infos[right.0].methods {
                    expected_methods.push(m.clone());
                }
                let mut ok = true;
                for m in expected_methods {
                    let cmp_methods =
                    |m_type: Symbol, m_protocol: Symbol, ctx: &SemanticContext| {
                        if m_type.name != m_protocol.name {
                                return false;
                            }
                            if let SymbolType::Function {
                                params: p_type,
                                ret: r_type,
                            } = m_type.ty
                            {
                                if let SymbolType::Function {
                                    params: p_protocol,
                                    ret: r_protocol,
                                } = m_protocol.ty
                                {
                                    if self.is_subtype_of(&ctx, r_type, r_protocol) {
                                        if p_protocol.len() == p_type.len() {
                                            let mut ok = true;
                                            for (param_type, param_protocol) in
                                            p_type.iter().zip(p_protocol.iter())
                                            {
                                                if !self.is_subtype_of(
                                                    &ctx,
                                                    *param_protocol,
                                                    *param_type,
                                                ) {
                                                    ok = false;
                                                }
                                            }
                                            return ok;
                                        }
                                    }
                                }
                            }
                            false
                        };
                        let mut left_methods: Vec<Symbol> = Vec::new();
                        let mut current = Some(left);
                        while let Some(id) = current {
                            for (_, m) in &ctx.types.infos[id.0].methods {
                            left_methods.push(m.clone());
                        }
                        current = ctx.types.infos[id.0].parent;
                    }
                    let mut ok2 = false;
                    for t_m in &left_methods {
                        if cmp_methods(t_m.clone(), m.clone(), ctx) {
                            ok2 = true;
                        }
                    }
                    if !ok2 {
                        ok = false;
                    }
                }
                return ok;
            }
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
        let method_name = method.name.clone();
        let already_existed = info.methods.contains_key(&method_name);
        info.methods.insert(method_name, method);
        !already_existed
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

    pub fn set_declared_constructor_params(
        &mut self,
        type_id: TypeId,
        params: Option<Vec<ConstructorParam>>,
    ) {
        if let Some(type_info) = self.infos.get_mut(type_id.0) {
            type_info.declared_constructor_params = params;
        }
    }

    pub fn set_effective_constructor_params(
        &mut self,
        type_id: TypeId,
        params: Vec<ConstructorParam>,
    ) {
        if let Some(type_info) = self.infos.get_mut(type_id.0) {
            let is_generic = params.iter().any(|p| p.ty.is_none());
            type_info.constructor_params = params;
            type_info.is_generic_template = is_generic;
        }
    }

    pub fn is_generic_template(&self, type_id: TypeId) -> bool {
        self.infos[type_id.0].is_generic_template
    }

    pub fn insert_protocol_placeholder(&mut self, name: String) -> Option<TypeId> {
        if self.by_name.contains_key(&name) {
            return None;
        }
        let id = TypeId(self.infos.len());
        self.infos.push(TypeInfo {
            kind: TypeKind::Protocol {
                parents: Vec::new(),
            },
            name: name.clone(),
            parent: None,
            declared_constructor_params: None,
            constructor_params: Vec::new(),
            attributes: HashMap::new(),
            methods: HashMap::new(),
            is_generic_template: false,
        });
        self.by_name.insert(name, id);
        Some(id)
    }
}
