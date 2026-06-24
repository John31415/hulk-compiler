use crate::ast::{Decl, DeclKind};
use crate::ast::{TypeAnnotation, TypeFeaturesKind};
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::hir::{
    TypedDecl, TypedDeclKind, TypedInheritInfo, TypedInheritInfoKind, TypedParam, TypedParamKind,
    TypedTypeFeature, TypedTypeFeatureKind,
};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};

impl SemanticAnalyzer {
    pub fn analyze_type(&mut self, type_decl: &Decl) -> TypedDecl {
        let object_type = self.resolve_builtin("Object");
        let (name, params, parent, features) = match &type_decl.node {
            DeclKind::Type {
                name,
                params,
                parent,
                features,
            } => (name, params, parent, features),
            _ => panic!("Expected type declaration"),
        };
        let current_type_id = self
            .ctx
            .types
            .resolve(name)
            .expect("Type should have already been registered in the first pass");
        self.ctx.current_type = Some(current_type_id);
        self.ctx.push_scope();
        let typed_params = params.as_ref().map(|param_list| {
            let mut t_params = Vec::new();
            for (param_name, param_type_opt) in param_list {
                let param_type_id = match param_type_opt {
                    Some(type_name) => match self.ctx.types.resolve_type(type_name) {
                        Some(id) if self.ctx.types.get(id).is_protocol() => {
                            let type_name = match type_name {
                                TypeAnnotation::Named { name, .. } => name,
                                TypeAnnotation::Star { name, .. } => name,
                            };
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::ProtocolNotAllowedAsParameterType {
                                        type_name: type_name.to_string(),
                                        param_name: param_name.to_string(),
                                    },
                                    type_decl.span,
                                )
                                .into(),
                            );
                            object_type
                        }
                        Some(id) => id,
                        None => {
                            let type_name = match type_name {
                                TypeAnnotation::Named { name, .. } => name,
                                TypeAnnotation::Star { name, .. } => name,
                            };
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::UnknownTypeInFunctionParameter {
                                        function: name.to_string(),
                                        param: param_name.to_string(),
                                        type_name: type_name.to_string(),
                                    },
                                    type_decl.span,
                                )
                                .into(),
                            );
                            object_type
                        }
                    },
                    None => object_type,
                };
                let param_symbol = Symbol {
                    name: param_name.clone(),
                    kind: SymbolKind::Parameter,
                    ty: SymbolType::Variable(param_type_id),
                    span: type_decl.span,
                };
                if !self.ctx.declare(param_symbol) {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::DuplicateConstructorParameter {
                                type_name: name.to_string(),
                                param: param_name.to_string(),
                            },
                            type_decl.span,
                        )
                        .into(),
                    );
                }
                let typed_param = TypedParam::new(
                    TypedParamKind {
                        name: param_name.clone(),
                        type_id: param_type_id,
                    },
                    type_decl.span,
                );
                t_params.push(typed_param);
            }
            t_params
        });
        let typed_inherit_info = parent.as_ref().map(|inherit_info| {
            let mut typed_args = Vec::new();
            let type_id = match self.ctx.types.resolve(&inherit_info.node.parent_name) {
                Some(parent_type_id) => {
                    let expected_parent_params = self
                        .ctx
                        .types
                        .get_constructor_params(parent_type_id)
                        .to_vec();
                    if let Some(actual_args) = &inherit_info.node.args {
                        if actual_args.len() != expected_parent_params.len() {
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::InvalidInheritanceArity {
                                        parent: inherit_info.node.parent_name.to_string(),
                                        expected: expected_parent_params.len(),
                                        found: actual_args.len(),
                                    },
                                    type_decl.span,
                                )
                                .into(),
                            );
                        }
                        for (i, arg_expr) in actual_args.iter().enumerate() {
                            let arg_expr_type = self.analyze_expr(arg_expr);
                            if i < expected_parent_params.len() {
                                let param_info = &expected_parent_params[i];
                                let expected_type_id = match param_info.ty {
                                    Some(id) => id,
                                    None => object_type,
                                };
                                if !self.ctx.types.is_subtype_of(
                                    &self.ctx,
                                    arg_expr_type.ty,
                                    expected_type_id,
                                ) {
                                    self.diagnostics.push(
                                        SemanticError::new(
                                            SemanticErrorKind::InheritanceArgumentTypeMismatch {
                                                parent: inherit_info.node.parent_name.to_string(),
                                                param: param_info.name.to_string(),
                                                expected: self
                                                    .ctx
                                                    .types
                                                    .get(expected_type_id)
                                                    .name
                                                    .clone(),
                                                found: self
                                                    .ctx
                                                    .types
                                                    .get(arg_expr_type.ty)
                                                    .name
                                                    .clone(),
                                            },
                                            arg_expr.span,
                                        )
                                        .into(),
                                    );
                                }
                            }
                            typed_args.push(arg_expr_type);
                        }
                    }
                    parent_type_id
                }
                None => {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::UnknownParentType {
                                child: name.to_string(),
                                parent: inherit_info.node.parent_name.to_string(),
                            },
                            type_decl.span,
                        )
                        .into(),
                    );
                    self.resolve_builtin("Object")
                }
            };
            let wrap_typed_args = (!typed_args.is_empty()).then_some(typed_args);
            TypedInheritInfo::new(
                TypedInheritInfoKind {
                    parent_type: type_id,
                    args: wrap_typed_args,
                },
                inherit_info.span,
            )
        });
        self.ctx.declare(Symbol {
            name: "self".to_string(),
            kind: SymbolKind::Variable,
            ty: SymbolType::Variable(current_type_id),
            span: type_decl.span,
        });
        let mut typed_features = Vec::new();
        for feature in features {
            if let TypeFeaturesKind::Attribute {
                name: attr_name,
                type_name,
                default,
            } = &feature.node
            {
                let typed_default = default
                    .as_ref()
                    .map(|init_expr| self.analyze_expr(init_expr));
                let expected_attr_type = match type_name {
                    Some(t_name) => self.ctx.types.resolve_type(t_name).unwrap_or_else(|| {
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
                    }),
                    None => typed_default
                        .as_ref()
                        .map(|td| td.ty)
                        .unwrap_or(object_type),
                };
                if let Some(ref default_val) = typed_default {
                    if !self
                        .ctx
                        .types
                        .is_subtype_of(&self.ctx, default_val.ty, expected_attr_type)
                    {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::AttributeTypeMismatch {
                                    attribute: attr_name.to_string(),
                                    expected: self.ctx.types.get(expected_attr_type).name.clone(),
                                    found: self.ctx.types.get(default_val.ty).name.clone(),
                                },
                                type_decl.span,
                            )
                            .into(),
                        );
                    }
                }
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
                if !self
                    .ctx
                    .types
                    .insert_attribute(current_type_id, attr_symbol)
                {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::DuplicateAttribute {
                                type_name: name.to_string(),
                                attribute: attr_name.to_string(),
                            },
                            feature.span,
                        )
                        .into(),
                    );
                }
                let typed_feature = TypedTypeFeature::new(
                    TypedTypeFeatureKind::Attribute {
                        name: attr_name.clone(),
                        type_id: final_attr_type,
                        default: typed_default,
                    },
                    feature.span,
                );
                typed_features.push(typed_feature);
            }
        }
        self.ctx.pop_scope();
        for feature in features {
            if let TypeFeaturesKind::Method {
                name: method_name,
                params: method_params,
                return_type,
                body,
            } = &feature.node
            {
                self.ctx.push_scope();
                self.ctx.declare(Symbol {
                    name: "self".to_string(),
                    kind: SymbolKind::Variable,
                    ty: SymbolType::Variable(current_type_id),
                    span: type_decl.span,
                });
                let mut current_method_param_types = Vec::new();
                let mut typed_params = Vec::new();
                for (p_name, p_type_opt) in method_params {
                    let p_type_id = match p_type_opt {
                        Some(t_name) => match self.ctx.types.resolve_type(t_name) {
                            Some(id) => {
                                let t_name = match t_name {
                                    TypeAnnotation::Named { name, .. } => name,
                                    TypeAnnotation::Star { name, .. } => name,
                                };
                                if self.ctx.types.get(id).is_protocol() {
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
                                    self.ctx.types.resolve("Object").unwrap()
                                } else {
                                    id
                                }
                            }
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
                    current_method_param_types.push(p_type_id);
                    self.ctx.declare(Symbol {
                        name: p_name.clone(),
                        kind: SymbolKind::Parameter,
                        ty: SymbolType::Variable(p_type_id),
                        span: feature.span,
                    });
                    let typed_param = TypedParam::new(
                        TypedParamKind {
                            name: p_name.clone(),
                            type_id: p_type_id,
                        },
                        feature.span,
                    );
                    typed_params.push(typed_param);
                }
                let declared_ret_id = match return_type {
                    Some(t_name) => {
                        Some(self.ctx.types.resolve_type(t_name).unwrap_or_else(|| {
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
                        }))
                    }
                    None => None,
                };
                if let Some(parent_info) = parent {
                    if let Some(parent_id) = self.ctx.types.resolve(&parent_info.node.parent_name) {
                        if let Some(parent_method) =
                            self.ctx.types.get_method(parent_id, method_name).cloned()
                        {
                            self.validate_method_override_arity_and_params(
                                method_name,
                                method_params,
                                &parent_method.ty,
                                type_decl.span,
                            );
                        }
                    }
                }
                self.ctx.current_function_return = declared_ret_id;
                self.ctx.current_method = Some(method_name.clone());
                let body_type = self.analyze_expr(body);
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
                                    body.span,
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
                if let Some(parent_info) = parent {
                    if let Some(parent_id) = self.ctx.types.resolve(&parent_info.node.parent_name) {
                        if let Some(parent_method) =
                            self.ctx.types.get_method(parent_id, method_name).cloned()
                        {
                            if let SymbolType::Function {
                                ret: parent_ret, ..
                            } = &parent_method.ty
                            {
                                if expected_ret_id != *parent_ret {
                                    self.diagnostics.push(
                                        SemanticError::new(
                                            SemanticErrorKind::InvalidOverrideReturnType {
                                                method: method_name.to_string(),
                                                found: self
                                                    .ctx
                                                    .types
                                                    .get(expected_ret_id)
                                                    .name
                                                    .clone(),
                                                expected: self
                                                    .ctx
                                                    .types
                                                    .get(*parent_ret)
                                                    .name
                                                    .clone(),
                                            },
                                            type_decl.span,
                                        )
                                        .into(),
                                    );
                                }
                            }
                        }
                    }
                }
                self.ctx.current_function_return = None;
                self.ctx.current_method = None;
                self.ctx.pop_scope();
                self.ctx.types.insert_method(
                    current_type_id,
                    Symbol {
                        name: method_name.clone(),
                        kind: SymbolKind::Function,
                        ty: SymbolType::Function {
                            params: typed_params.iter().map(|p| p.node.type_id).collect(),
                            ret: expected_ret_id,
                        },
                        span: feature.span,
                    },
                );
                let typed_feature = TypedTypeFeature::new(
                    TypedTypeFeatureKind::Method {
                        name: method_name.clone(),
                        params: typed_params,
                        return_type: expected_ret_id,
                        body: body_type,
                    },
                    feature.span,
                );
                typed_features.push(typed_feature);
            }
        }
        self.ctx.current_type = None;
        TypedDecl::new(
            TypedDeclKind::Type {
                name: name.clone(),
                params: typed_params,
                parent: typed_inherit_info,
                features: typed_features,
                type_id: current_type_id,
            },
            type_decl.span,
        )
    }

    pub fn validate_method_override_arity_and_params(
        &mut self,
        method_name: &str,
        current_params: &[(String, Option<TypeAnnotation>)],
        parent_method_sig: &SymbolType,
        span: Span,
    ) {
        if let SymbolType::Function {
            params: parent_params,
            ..
        } = parent_method_sig
        {
            if current_params.len() != parent_params.len() {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::InvalidOverrideArity {
                            method: method_name.to_string(),
                            found: current_params.len(),
                            expected: parent_params.len(),
                        },
                        span,
                    )
                    .into(),
                );
            }
            for (i, (p_name, p_type_opt)) in current_params.iter().enumerate() {
                if i >= parent_params.len() {
                    break;
                }
                let current_p_id = p_type_opt
                    .as_ref()
                    .and_then(|t| self.ctx.types.resolve_type(t))
                    .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                if current_p_id != parent_params[i] {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::InvalidOverrideParameterType {
                                method: method_name.to_string(),
                                param_name: p_name.to_string(),
                                found: self.ctx.types.get(current_p_id).name.clone(),
                                expected: self.ctx.types.get(parent_params[i]).name.clone(),
                            },
                            span,
                        )
                        .into(),
                    );
                }
            }
        }
    }
}
