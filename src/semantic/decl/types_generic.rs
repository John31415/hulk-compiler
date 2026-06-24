use crate::ast::{DeclKind, TypeAnnotation};
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::context::GenericInstanceKey;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{
    TypedDecl, TypedDeclKind, TypedInheritInfo, TypedInheritInfoKind, TypedParam, TypedParamKind,
    TypedTypeFeature, TypedTypeFeatureKind,
};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::types::{ConstructorParam, TypeId};

impl SemanticAnalyzer {
    pub fn instantiate_generic_type(
        &mut self,
        name: &str,
        concrete_types: &[TypeId],
        call_site_span: Span,
    ) -> Option<TypeId> {
        let type_decl = match self.ctx.generic_type_decls.get(name).cloned() {
            Some(decl) => decl,
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UnknownType {
                            name: name.to_string(),
                        },
                        call_site_span,
                    )
                    .into(),
                );
                return None;
            }
        };
        let (decl_name, parent, features) = match &type_decl.node {
            DeclKind::Type {
                name,
                parent,
                features,
                ..
            } => (name, parent, features),
            _ => panic!("Expected type declaration in generic_type_decls"),
        };
        let key: GenericInstanceKey = (decl_name.clone(), concrete_types.to_vec());
        let molde_type_id = self
            .ctx
            .types
            .resolve(decl_name)
            .expect("generic type template must be registered in TypeTable");
        let effective_param_names: Vec<String> = self
            .ctx
            .types
            .get_constructor_params(molde_type_id)
            .iter()
            .map(|p| p.name.clone())
            .collect();
        if effective_param_names.len() != concrete_types.len() {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidConstructorArity {
                        type_name: decl_name.to_string(),
                        expected: effective_param_names.len(),
                        found: concrete_types.len(),
                    },
                    call_site_span,
                )
                .into(),
            );
            return None;
        }
        self.ctx.mark_type_in_progress(key.clone());
        let outer_current_type = self.ctx.current_type;
        let outer_current_method = self.ctx.current_method.clone();
        let outer_current_function_return = self.ctx.current_function_return;
        let mangled_name = self.ctx.mangle_instance_name(decl_name, concrete_types);
        let new_type_id = self
            .ctx
            .types
            .insert_instantiation(mangled_name.clone(), None);
        let new_constructor_params: Vec<ConstructorParam> = effective_param_names
            .iter()
            .zip(concrete_types.iter())
            .map(|(pname, &ptype)| ConstructorParam {
                name: pname.clone(),
                ty: Some(ptype),
            })
            .collect();
        self.ctx
            .types
            .set_declared_constructor_params(new_type_id, Some(new_constructor_params.clone()));
        self.ctx
            .types
            .set_effective_constructor_params(new_type_id, new_constructor_params);
        self.ctx.current_type = Some(new_type_id);
        self.ctx.push_scope();
        let mut typed_params = Vec::new();
        for (pname, &ptype) in effective_param_names.iter().zip(concrete_types.iter()) {
            let param_symbol = Symbol {
                name: pname.clone(),
                kind: SymbolKind::Parameter,
                ty: SymbolType::Variable(ptype),
                span: type_decl.span,
            };
            let _ = self.ctx.declare(param_symbol);
            typed_params.push(TypedParam::new(
                TypedParamKind {
                    name: pname.clone(),
                    type_id: ptype,
                },
                type_decl.span,
            ));
        }
        let typed_inherit_info = self.resolve_instantiated_parent(
            decl_name,
            parent.as_ref(),
            &type_decl.span,
            call_site_span,
            concrete_types,
        );
        self.ctx.current_type = Some(new_type_id);
        let parent_type_id_for_layout = typed_inherit_info.as_ref().map(|p| p.node.parent_type);
        self.ctx
            .types
            .set_parent(new_type_id, parent_type_id_for_layout);
        self.ctx.declare(Symbol {
            name: "self".to_string(),
            kind: SymbolKind::Variable,
            ty: SymbolType::Variable(new_type_id),
            span: type_decl.span,
        });
        let object_type = self.resolve_builtin("Object");
        let mut typed_features = Vec::new();
        for feature in features {
            let TypeFeaturesKindRef::Attribute {
                attr_name,
                type_name,
                default,
            } = classify_feature(feature)
            else {
                continue;
            };

            let declared_attr_type = match type_name {
                Some(t_name) => Some(self.ctx.types.resolve_type(t_name).unwrap_or_else(|| {
                    let t_name = match t_name {
                        TypeAnnotation::Named { name, .. } => name,
                        TypeAnnotation::Star { name, .. } => name,
                    };
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::UnknownTypeInAttribute {
                                type_name: t_name.to_string(),
                                attribute: attr_name.to_string(),
                            },
                            type_decl.span,
                        )
                        .into(),
                    );
                    object_type
                })),
                None => None,
            };
            let (expected_attr_type, typed_default) = match declared_attr_type {
                Some(declared) => {
                    let typed_default = default.as_ref().map(|init_expr| {
                        let inferred_type = self.analyze_expr(init_expr);
                        if !self
                            .ctx
                            .types
                            .is_subtype_of(&self.ctx, inferred_type.ty, declared)
                        {
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::AttributeTypeMismatch {
                                        attribute: attr_name.to_string(),
                                        expected: self.ctx.types.get(declared).name.clone(),
                                        found: self.ctx.types.get(inferred_type.ty).name.clone(),
                                    },
                                    type_decl.span,
                                )
                                .into(),
                            );
                        }
                        inferred_type
                    });
                    (declared, typed_default)
                }
                None => match default.as_ref() {
                    Some(init_expr) => {
                        let inferred_type = self.analyze_expr(init_expr);
                        let ty = inferred_type.ty;
                        (ty, Some(inferred_type))
                    }
                    None => (object_type, None),
                },
            };
            let final_attr_type = if self.ctx.types.get(expected_attr_type).is_protocol() {
                typed_default
                    .as_ref()
                    .map(|td| td.ty)
                    .unwrap_or(expected_attr_type)
            } else {
                expected_attr_type
            };
            let attr_symbol = Symbol {
                name: attr_name.clone(),
                kind: SymbolKind::Attribute,
                ty: SymbolType::Variable(final_attr_type),
                span: feature.span,
            };
            if !self.ctx.types.insert_attribute(new_type_id, attr_symbol) {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::DuplicateAttribute {
                            type_name: decl_name.to_string(),
                            attribute: attr_name.to_string(),
                        },
                        feature.span,
                    )
                    .into(),
                );
            }
            typed_features.push(TypedTypeFeature::new(
                TypedTypeFeatureKind::Attribute {
                    name: attr_name.clone(),
                    type_id: final_attr_type,
                    default: typed_default,
                },
                feature.span,
            ));
        }
        for feature in features {
            let TypeFeaturesKindRef::Method {
                method_name,
                method_params,
                return_type,
                ..
            } = classify_feature(feature)
            else {
                continue;
            };
            let has_unannotated_param = method_params.iter().any(|(_, t)| t.is_none());
            if has_unannotated_param {
                if let Some(parent_id) = parent_type_id_for_layout {
                    if self.ctx.types.get_method(parent_id, method_name).is_some() {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::GenericMethodOverrideNotAllowed {
                                    method: method_name.to_string(),
                                    type_name: mangled_name.clone(),
                                },
                                feature.span,
                            )
                            .into(),
                        );
                    }
                }
                self.ctx.register_pending_generic_method(
                    new_type_id,
                    method_name.to_string(),
                    feature.clone(),
                );
                continue;
            }
            self.ctx.push_scope();
            self.ctx.declare(Symbol {
                name: "self".to_string(),
                kind: SymbolKind::Variable,
                ty: SymbolType::Variable(new_type_id),
                span: type_decl.span,
            });
            let mut typed_method_params = Vec::new();
            for (p_name, p_type_opt) in method_params {
                let p_type_id = match p_type_opt {
                    Some(t_name) => match self.ctx.types.resolve_type(t_name) {
                        Some(id) if self.ctx.types.get(id).is_protocol() => {
                            let t_name = match t_name {
                                TypeAnnotation::Named { name, .. } => name,
                                TypeAnnotation::Star { name, .. } => name,
                            };
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::ProtocolNotAllowedAsParameterType {
                                        type_name: t_name.to_string(),
                                        param_name: p_name.to_string(),
                                    },
                                    feature.span,
                                )
                                .into(),
                            );
                            object_type
                        }
                        Some(id) => id,
                        None => {
                            let t_name = match t_name {
                                TypeAnnotation::Named { name, .. } => name,
                                TypeAnnotation::Star { name, .. } => name,
                            };
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::UnknownTypeInMethodParameter {
                                        method: method_name.to_string(),
                                        param: p_name.to_string(),
                                        type_name: t_name.to_string(),
                                    },
                                    feature.span,
                                )
                                .into(),
                            );
                            object_type
                        }
                    },
                    None => object_type,
                };
                self.ctx.declare(Symbol {
                    name: p_name.clone(),
                    kind: SymbolKind::Parameter,
                    ty: SymbolType::Variable(p_type_id),
                    span: feature.span,
                });
                typed_method_params.push(TypedParam::new(
                    TypedParamKind {
                        name: p_name.clone(),
                        type_id: p_type_id,
                    },
                    feature.span,
                ));
            }
            let declared_ret_id = match return_type {
                Some(t_name) => Some(self.ctx.types.resolve_type(t_name).unwrap_or_else(|| {
                    let t_name = match t_name {
                        TypeAnnotation::Named { name, .. } => name,
                        TypeAnnotation::Star { name, .. } => name,
                    };
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::UnknownReturnTypeInMethod {
                                method: method_name.to_string(),
                                type_name: t_name.to_string(),
                            },
                            feature.span,
                        )
                        .into(),
                    );
                    object_type
                })),
                None => None,
            };
            let parent_method_for_override = parent_type_id_for_layout
                .and_then(|parent_id| self.ctx.types.get_method(parent_id, method_name).cloned());
            if let Some(parent_method) = &parent_method_for_override {
                self.validate_method_override_arity_and_params(
                    method_name,
                    method_params,
                    &parent_method.ty,
                    type_decl.span,
                );
            }
            self.ctx.current_function_return = declared_ret_id;
            self.ctx.current_method = Some(method_name.to_string());
            let body_type = self.analyze_expr(method_body(feature));
            let expected_ret_id = match declared_ret_id {
                Some(declared) => {
                    if !self
                        .ctx
                        .types
                        .is_subtype_of(&self.ctx, body_type.ty, declared)
                    {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::MethodReturnTypeMismatch {
                                    method: method_name.to_string(),
                                    expected: self.ctx.types.get(declared).name.clone(),
                                    found: self.ctx.types.get(body_type.ty).name.clone(),
                                },
                                body_type.span,
                            )
                            .into(),
                        );
                    }
                    if self.ctx.types.get(declared).is_protocol() {
                        body_type.ty
                    } else {
                        declared
                    }
                }
                None => body_type.ty,
            };
            if let Some(parent_method) = &parent_method_for_override {
                if let SymbolType::Function {
                    ret: parent_ret, ..
                } = &parent_method.ty
                {
                    if expected_ret_id != *parent_ret {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::InvalidOverrideReturnType {
                                    method: method_name.to_string(),
                                    found: self.ctx.types.get(expected_ret_id).name.clone(),
                                    expected: self.ctx.types.get(*parent_ret).name.clone(),
                                },
                                type_decl.span,
                            )
                            .into(),
                        );
                    }
                }
            }
            self.ctx.current_function_return = None;
            self.ctx.current_method = None;
            self.ctx.pop_scope();
            if !self.ctx.types.insert_method(
                new_type_id,
                Symbol {
                    name: method_name.to_string(),
                    kind: SymbolKind::Function,
                    ty: SymbolType::Function {
                        params: typed_method_params.iter().map(|p| p.node.type_id).collect(),
                        ret: expected_ret_id,
                    },
                    span: feature.span,
                },
            ) {}
            typed_features.push(TypedTypeFeature::new(
                TypedTypeFeatureKind::Method {
                    name: method_name.to_string(),
                    params: typed_method_params,
                    return_type: expected_ret_id,
                    body: body_type,
                },
                feature.span,
            ));
        }
        self.ctx.pop_scope();
        self.ctx.current_type = outer_current_type;
        self.ctx.current_method = outer_current_method;
        self.ctx.current_function_return = outer_current_function_return;
        self.ctx.unmark_type_in_progress(&key);
        let typed_decl = TypedDecl::new(
            TypedDeclKind::Type {
                name: mangled_name,
                params: Some(typed_params),
                parent: typed_inherit_info,
                features: typed_features,
                type_id: new_type_id,
            },
            type_decl.span,
        );
        self.ctx.insert_type_instance(key, new_type_id, typed_decl);
        Some(new_type_id)
    }

    fn resolve_instantiated_parent(
        &mut self,
        child_name: &str,
        parent: Option<&crate::ast::InheritInfo>,
        type_decl_span: &crate::lexer::span::Span,
        call_site_span: Span,
        child_concrete_types: &[TypeId],
    ) -> Option<TypedInheritInfo> {
        let inherit_info = parent?;
        let object_type = self.resolve_builtin("Object");
        let parent_name = &inherit_info.node.parent_name;
        let parent_molde_id = match self.ctx.types.resolve(parent_name) {
            Some(id) => id,
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UnknownParentType {
                            child: child_name.to_string(),
                            parent: parent_name.to_string(),
                        },
                        *type_decl_span,
                    )
                    .into(),
                );
                return Some(TypedInheritInfo::new(
                    TypedInheritInfoKind {
                        parent_type: self.resolve_builtin("Object"),
                        args: None,
                    },
                    inherit_info.span,
                ));
            }
        };
        let actual_args = inherit_info
            .node
            .args
            .as_ref()
            .map(|v| v.as_slice())
            .unwrap_or_default();
        if !self.ctx.types.is_generic_template(parent_molde_id) {
            let expected_parent_params = self
                .ctx
                .types
                .get_constructor_params(parent_molde_id)
                .to_vec();
            if actual_args.len() != expected_parent_params.len() {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::InvalidInheritanceArity {
                            parent: parent_name.to_string(),
                            expected: expected_parent_params.len(),
                            found: actual_args.len(),
                        },
                        *type_decl_span,
                    )
                    .into(),
                );
            }
            let mut typed_args = Vec::new();
            for (i, arg_expr) in actual_args.iter().enumerate() {
                let arg_expr_type = self.analyze_expr(arg_expr);
                if i < expected_parent_params.len() {
                    let expected_type_id = expected_parent_params[i].ty.unwrap_or(object_type);
                    if !self
                        .ctx
                        .types
                        .is_subtype_of(&self.ctx, arg_expr_type.ty, expected_type_id)
                    {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::InheritanceArgumentTypeMismatch {
                                    parent: parent_name.to_string(),
                                    param: expected_parent_params[i].name.clone(),
                                    expected: self.ctx.types.get(expected_type_id).name.clone(),
                                    found: self.ctx.types.get(arg_expr_type.ty).name.clone(),
                                },
                                arg_expr.span,
                            )
                            .into(),
                        );
                    }
                }
                typed_args.push(arg_expr_type);
            }
            let wrap_typed_args = (!typed_args.is_empty()).then_some(typed_args);
            return Some(TypedInheritInfo::new(
                TypedInheritInfoKind {
                    parent_type: parent_molde_id,
                    args: wrap_typed_args,
                },
                inherit_info.span,
            ));
        }
        if inherit_info.node.args.is_none() {
            let parent_key: GenericInstanceKey =
                (parent_name.clone(), child_concrete_types.to_vec());
            let resolved_parent_type_id =
                if let Some(existing) = self.ctx.get_type_instance(&parent_key) {
                    existing
                } else if self.ctx.is_type_in_progress(&parent_key) {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::UnknownParentType {
                                child: child_name.to_string(),
                                parent: parent_name.to_string(),
                            },
                            *type_decl_span,
                        )
                        .into(),
                    );
                    object_type
                } else {
                    match self.instantiate_generic_type(
                        parent_name,
                        child_concrete_types,
                        call_site_span,
                    ) {
                        Some(id) => id,
                        None => object_type,
                    }
                };
            return Some(TypedInheritInfo::new(
                TypedInheritInfoKind {
                    parent_type: resolved_parent_type_id,
                    args: None,
                },
                inherit_info.span,
            ));
        }
        let mut typed_args = Vec::new();
        let mut parent_concrete_types = Vec::new();
        for arg_expr in actual_args {
            let arg_expr_type = self.analyze_expr(arg_expr);
            parent_concrete_types.push(arg_expr_type.ty);
            typed_args.push(arg_expr_type);
        }
        let parent_key: GenericInstanceKey = (parent_name.clone(), parent_concrete_types.clone());
        let resolved_parent_type_id = if let Some(existing) =
            self.ctx.get_type_instance(&parent_key)
        {
            existing
        } else if self.ctx.is_type_in_progress(&parent_key) {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::UnknownParentType {
                        child: child_name.to_string(),
                        parent: parent_name.to_string(),
                    },
                    *type_decl_span,
                )
                .into(),
            );
            object_type
        } else {
            match self.instantiate_generic_type(parent_name, &parent_concrete_types, call_site_span)
            {
                Some(id) => id,
                None => object_type,
            }
        };
        let wrap_typed_args = (!typed_args.is_empty()).then_some(typed_args);
        Some(TypedInheritInfo::new(
            TypedInheritInfoKind {
                parent_type: resolved_parent_type_id,
                args: wrap_typed_args,
            },
            inherit_info.span,
        ))
    }
}

enum TypeFeaturesKindRef<'a> {
    Attribute {
        attr_name: &'a String,
        type_name: &'a Option<TypeAnnotation>,
        default: &'a Option<crate::ast::Expr>,
    },
    Method {
        method_name: &'a String,
        method_params: &'a Vec<(String, Option<TypeAnnotation>)>,
        return_type: &'a Option<TypeAnnotation>,
    },
}

fn classify_feature(feature: &crate::ast::TypeFeatures) -> TypeFeaturesKindRef<'_> {
    match &feature.node {
        crate::ast::TypeFeaturesKind::Attribute {
            name,
            type_name,
            default,
        } => TypeFeaturesKindRef::Attribute {
            attr_name: name,
            type_name,
            default,
        },
        crate::ast::TypeFeaturesKind::Method {
            name,
            params,
            return_type,
            ..
        } => TypeFeaturesKindRef::Method {
            method_name: name,
            method_params: params,
            return_type,
        },
    }
}

fn method_body(feature: &crate::ast::TypeFeatures) -> &crate::ast::Expr {
    match &feature.node {
        crate::ast::TypeFeaturesKind::Method { body, .. } => body,
        _ => panic!("method_body called on a non-Method TypeFeatures"),
    }
}
