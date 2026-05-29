use crate::lexer::span::Span;
use crate::semantic::types::ConstructorParam;
use super::SemanticAnalyzer;
use super::symbols::{Symbol, SymbolKind, SymbolType};
use crate::ast::TypeFeaturesKind;
use super::types::TypeId;
use crate::{
    ast::{Decl, DeclKind},
    diagnostics::Diagnostic,
};

impl SemanticAnalyzer {
    pub fn collect_declarations(&mut self, decls: &[Decl]) {
        for decl in decls {
            match &decl.node {
                DeclKind::Function { name, .. } => {
                    let ok = self.ctx.declare(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        ty: SymbolType::Unknown,
                        span: decl.span,
                    });
                    if !ok {
                        self.diagnostics.push(Diagnostic::error(
                            format!("duplicate function '{}'", name),
                            decl.span,
                        ));
                    }
                }
                DeclKind::Type { name, .. } => {
                    let ok = self.ctx.types.insert(name.clone(), None).is_some();
                    if !ok {
                        self.diagnostics.push(Diagnostic::error(
                            format!("duplicate type '{}'", name),
                            decl.span,
                        ));
                    }
                }
            }
        }
    }

    pub fn check_declarations(&mut self, decls: &[Decl]) {
        self.register_signatures(decls);
        self.check_circular_inheritance(decls);
        for decl in decls {
            match &decl.node {
                DeclKind::Function { .. } => {
                    self.check_function(decl);
                }
                DeclKind::Type { .. } => {
                    self.check_type(decl);
                }
            }
        }
    }

    fn register_signatures(&mut self, decls: &[Decl]) {
        for decl in decls {
            match &decl.node {
                DeclKind::Function { name, params, return_type, .. } => {
                    let mut param_types = Vec::new();
                    for (_, param_type_opt) in params {
                        let p_type = param_type_opt.as_ref() 
                            .and_then(|t| self.ctx.types.resolve(t))
                            .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                        param_types.push(p_type);
                    }
                    let ret_type = return_type.as_ref()
                        .and_then(|t| self.ctx.types.resolve(t))
                        .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                    let updated_symbol = Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        ty: SymbolType::Function { params: param_types, ret: ret_type },
                        span: decl.span,
                    };
                    self.ctx.declare(updated_symbol);
                }
                DeclKind::Type { name, params, parent, features } => {
                    let current_type_id = self.ctx.types.resolve(name)
                        .expect("Type should have already been registered in collect_declarations");
                    if let Some(inherit_info) = parent {
                        if let Some(parent_type_id) = self.ctx.types.resolve(&inherit_info.node.parent_name) {
                            let parent_name = &inherit_info.node.parent_name;
                            if parent_name == "Number" || parent_name == "String" || parent_name == "Boolean" {
                                self.diagnostics.push(
                                    Diagnostic::error(
                                        format!("Type '{}' cannot inherit from primitive type '{}'", name, parent_name),
                                        inherit_info.span,
                                    )
                                );
                                self.ctx.types.set_parent(current_type_id, None);
                            } else {
                                self.ctx.types.set_parent(current_type_id, Some(parent_type_id));
                            }
                        }
                    }
                    let mut constructor_params = Vec::new();
                    if let Some(param_list) = params {
                        for(p_name, p_type_opt) in param_list {
                            let p_type = p_type_opt.as_ref().and_then(|t| self.ctx.types.resolve(t));
                            constructor_params.push(
                                ConstructorParam {
                                    name: p_name.clone(),
                                    ty: p_type,
                                }
                            );
                        }
                    }
                    self.ctx.types.set_constructor_params(current_type_id, constructor_params);
                    for feature in features {
                        if let TypeFeaturesKind::Method { name: method_name, params: method_params, return_type, .. } = &feature.node {
                            let mut p_types = Vec::new();
                            for (_, p_type_opt) in method_params {
                                let p_id = p_type_opt.as_ref()
                                    .and_then(|t| self.ctx.types.resolve(t))
                                    .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                                p_types.push(p_id);
                            }
                            let r_id = return_type.as_ref()
                                .and_then(|t| self.ctx.types.resolve(t))
                                .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                            let method_symbol = Symbol {
                                name: method_name.clone(),
                                kind: SymbolKind::Function,
                                ty: SymbolType::Function { params: p_types, ret: r_id },
                                span: feature.span,
                            };
                            self.ctx.types.insert_method(current_type_id, method_symbol);
                        }
                    }
                }
            }
        }
    }

    fn check_circular_inheritance(&mut self, decls: &[Decl]) {
        for decl in decls {
            if let DeclKind::Type { name, .. } = &decl.node {
                if let Some(start_id) = self.ctx.types.resolve(name) {
                    let mut current  = self.ctx.types.get_parent(start_id);
                    let mut visited = vec![start_id];
                    while let Some(parent_id) = current {
                        if visited.contains(&parent_id) {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    format!("Circular inheritance detected in type '{}'", name),
                                    decl.span,
                                )
                            );
                            self.ctx.types.set_parent(start_id, None);
                            break;
                        }
                        visited.push(parent_id);
                        current = self.ctx.types.get_parent(parent_id);
                    }
                }
            }
        }
    }

    fn check_function(&mut self, function_decl: &Decl) {
        if let DeclKind::Function {
            name,
            params,
            return_type,
            body,
        } = &function_decl.node
        {
            self.ctx.push_scope();
            for (param_name, param_type_opt) in params {
                let param_type_id = match param_type_opt {
                    Some(type_name) => match self.ctx.types.resolve(type_name) {
                        Some(id) => id,
                        None => {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "Non-existent type '{}' for parameter '{}'",
                                    type_name, param_name
                                ),
                                function_decl.span,
                            ));
                            self.ctx.types.resolve("Object").unwrap()
                        }
                    },
                    None => self.ctx.types.resolve("Object").unwrap(),
                };
                let param_symbol = Symbol {
                    name: param_name.clone(),
                    kind: SymbolKind::Parameter,
                    ty: SymbolType::Variable(param_type_id),
                    span: function_decl.span,
                };
                if !self.ctx.declare(param_symbol) {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "Parameter '{}' is already defined in function '{}'",
                            param_name, name
                        ),
                        function_decl.span,
                    ));
                }
            }
            let expected_return = match return_type {
                Some(type_name) => match self.ctx.types.resolve(type_name) {
                    Some(id) => id,
                    None => {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "Non-existent return type '{}' in function '{}'",
                                type_name, name
                            ),
                            function_decl.span,
                        ));
                        self.ctx.types.resolve("Object").unwrap()
                    }
                },
                None => self.ctx.types.resolve("Object").unwrap(),
            };
            self.ctx.current_function_return = Some(expected_return);
            let body_type = self.check_expr(body);
            if !self.ctx.types.is_subtype_of(body_type, expected_return) {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "Type mismatch in '{}': Body returns '{}' but expected '{}'",
                        name,
                        self.ctx.types.get(body_type).name,
                        self.ctx.types.get(expected_return).name
                    ),
                    body.span,
                ));
            }
            self.ctx.current_function_return = None;
            self.ctx.pop_scope();
        }
    }

    fn check_type(&mut self, type_decl: &Decl) {
        if let DeclKind::Type { name, params, parent, features } = &type_decl.node {
            let current_type_id = self.ctx.types.resolve(name).expect("Type should have already been registered in the first pass");
            self.ctx.current_type = Some(current_type_id);
            self.ctx.push_scope();
            if let Some(param_list) = params {
                for (param_name, param_type_opt) in param_list {
                    let param_type_id = match param_type_opt {
                        Some(type_name) => match self.ctx.types.resolve(type_name) {
                            Some(id) => id,
                            None => {
                                self.diagnostics.push(Diagnostic::error(
                                    format!("Non-existent type '{}' for parameter '{}' in '{}'", type_name, param_name, name),
                                    type_decl.span,
                                ));
                                self.ctx.types.resolve("Object").unwrap()
                            },
                        },
                        None => self.ctx.types.resolve("Object").unwrap()
                    };
                    let param_symbol = Symbol {
                        name: param_name.clone(),
                        kind: SymbolKind::Parameter,
                        ty: SymbolType::Variable(param_type_id),
                        span: type_decl.span,
                    };
                    if !self.ctx.declare(param_symbol) {
                        self.diagnostics.push(Diagnostic::error(
                            format!("Parameter '{}' is already defined in the constructor of type '{}'", param_name, name),
                            type_decl.span,
                        ));
                    }
                }
            }
            if let Some(inherit_info) = parent {
                match self.ctx.types.resolve(&inherit_info.node.parent_name) {
                    Some(parent_type_id) => {
                        let expected_parent_params = self.ctx.types.get_constructor_params(parent_type_id).to_vec();
                        let actual_args = inherit_info.node.args.as_ref().map(|v| v.as_slice()).unwrap_or_default();
                        if actual_args.len() != expected_parent_params.len() {
                            self.diagnostics.push(Diagnostic::error(
                                format!("Type '{}' expects '{}' arguments in its constructor, but '{}' were passed", inherit_info.node.parent_name, expected_parent_params.len(), actual_args.len()),
                                type_decl.span,
                            ));
                        }
                        for (i, arg_expr) in actual_args.iter().enumerate() {
                            let inferred_type_id = self.check_expr(arg_expr);
                            if i < expected_parent_params.len() {
                                let param_info = &expected_parent_params[i];
                                let expected_type_id = match param_info.ty {
                                    Some(id) => id,
                                    None => self.ctx.types.resolve("Object").unwrap(),
                                };
                                if !self.ctx.types.is_subtype_of(inferred_type_id, expected_type_id) {
                                    self.diagnostics.push(Diagnostic::error(
                                        format!("Incompatible type in inheritance argument '{}' of '{}': Inferred '{}' but expected '{}'", param_info.name, inherit_info.node.parent_name, self.ctx.types.get(inferred_type_id).name, self.ctx.types.get(expected_type_id).name),
                                        arg_expr.span,
                                    ));
                                }
                            }
                        }
                    },
                    None => {
                        self.diagnostics.push(Diagnostic::error(
                            format!("Type '{}' attempts to inherit from non-existent type '{}'", name, inherit_info.node.parent_name),
                            type_decl.span,
                        ));
                    }
                }
            }
            self.ctx.declare(
                Symbol {
                    name: "self".to_string(),
                    kind: SymbolKind::Variable,
                    ty: SymbolType::Variable(current_type_id),
                    span: type_decl.span,
                }
            );
            for feature in features {
                if let TypeFeaturesKind::Attribute { name: attr_name, type_name, default } = &feature.node {
                    let expected_attr_type = match type_name {
                        Some(t_name) => self.ctx.types.resolve(t_name).unwrap_or_else(|| {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "Non-existent type '{}' for attribute '{}'",
                                    t_name, attr_name
                                ),
                                type_decl.span,
                            ));
                            self.ctx.types.resolve("Object").unwrap()
                        }),
                        None => self.ctx.types.resolve("Object").unwrap()
                    };
                    if let Some(init_expr) = default {
                        let inferred_type = self.check_expr(init_expr);
                        if !self.ctx.types.is_subtype_of(inferred_type, expected_attr_type) {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "Incompatible type in attribute '{}': cannot assign '{}' to '{}'",
                                    attr_name, 
                                    self.ctx.types.get(inferred_type).name,
                                    self.ctx.types.get(expected_attr_type).name,
                                ),
                                type_decl.span,
                            ));
                        }
                    }
                    let attr_symbol = Symbol {
                        name: attr_name.clone(),
                        kind: SymbolKind::Attribute,
                        ty: SymbolType::Variable(expected_attr_type),
                        span: feature.span,
                    };
                    if !self.ctx.types.insert_attribute(current_type_id, attr_symbol) {
                        self.diagnostics.push(Diagnostic::error(
                            format!("Attribute '{}' is already defined in type '{}' or its ancestors", attr_name, name),
                            feature.span,
                        ));
                    }
                }
            }
            for feature in features {
                if let TypeFeaturesKind::Method { name: method_name, params: method_params, return_type, body } = &feature.node {
                    self.ctx.push_scope();
                    self.ctx.declare(
                        Symbol {
                            name: "self".to_string(),
                            kind: SymbolKind::Variable,
                            ty: SymbolType::Variable(current_type_id),
                            span: type_decl.span,
                        }
                    );
                    let mut current_method_param_types = Vec::new();
                    for (p_name, p_type_opt) in method_params {
                        let p_type_id = match p_type_opt {
                            Some(t_name) => match self.ctx.types.resolve(t_name) {
                                Some(id) => id,
                                None => {
                                    self.diagnostics.push(
                                        Diagnostic::error(
                                            format!("Non-existent type '{}' for parameter '{}' in method '{}'", t_name, p_name, method_name),
                                            feature.span,
                                        )
                                    );
                                    self.ctx.types.resolve("Object").unwrap()
                                }
                            },
                            None => self.ctx.types.resolve("Object").unwrap(),
                        };
                        current_method_param_types.push(p_type_id);
                        self.ctx.declare(
                            Symbol {
                                name: p_name.clone(),
                                kind: SymbolKind::Parameter,
                                ty: SymbolType::Variable(p_type_id),
                                span: feature.span,
                            }
                        );
                    }
                    let expected_ret_id = match return_type {
                        Some(t_name) => match self.ctx.types.resolve(t_name) {
                            Some(id) => id,
                            None => {
                                self.diagnostics.push(
                                    Diagnostic::error(
                                        format!("Non-existent return type '{}' in method '{}'", t_name, method_name),
                                        feature.span,
                                    )
                                );
                                self.ctx.types.resolve("Object").unwrap()
                            }
                        },
                        None => self.ctx.types.resolve("Object").unwrap(),
                    };
                    if let Some(parent_info) = parent {
                        if let Some(parent_id) = self.ctx.types.resolve(&parent_info.node.parent_name) {
                            if let Some(parent_method) = self.ctx.types.get_method(parent_id, method_name).cloned() {
                                self.validate_method_override(method_name, method_params, expected_ret_id, &parent_method.ty, type_decl.span);
                            }
                        }
                    }
                    self.ctx.current_function_return = Some(expected_ret_id);
                    self.ctx.current_method = Some(method_name.clone());
                    let body_type = self.check_expr(body);
                    if !self.ctx.types.is_subtype_of(body_type, expected_ret_id) {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "Method '{}' must return '{}' but its body returns '{}'",
                                method_name, 
                                self.ctx.types.get(expected_ret_id).name,
                                self.ctx.types.get(body_type).name,
                            ),
                            body.span,
                        ));
                    }
                    self.ctx.current_function_return = None;
                    self.ctx.current_method = None;
                    self.ctx.pop_scope();
                }
            }
            self.ctx.pop_scope();
            self.ctx.current_type = None;
        }
    }

    fn validate_method_override(&mut self, method_name: &str, current_params: &[(String, Option<String>)], current_ret: TypeId, parent_method_sig: &SymbolType, span: Span) {
        if let SymbolType::Function { params: parent_params, ret: parent_ret } = parent_method_sig {
            if current_params.len() != parent_params.len() {
                self.diagnostics.push(
                    Diagnostic::error(
                        format!("Override method '{}' has a different number of parameters the its parent class", method_name),
                        span,
                    )
                );
                return;
            }
            if current_ret != *parent_ret {
                self.diagnostics.push(
                    Diagnostic::error(
                        format!("Return type of override method '{}' does not match that of the parent class", method_name),
                        span,
                    )
                );
            }
            for (i, (_, p_type_opt)) in current_params.iter().enumerate() {
                let current_p_id = p_type_opt.as_ref().and_then(|t| self.ctx.types.resolve(t)).unwrap_or_else(|| {
                    self.ctx.types.resolve("Object").unwrap()
                });
                if current_p_id != parent_params[i] {
                    self.diagnostics.push(
                        Diagnostic::error(
                            format!("Parameter '{}' of override method '{}' does not match the type of the parent method", i + 1, method_name),
                            span,
                        )
                    );
                }
            }
        }
    }
}
