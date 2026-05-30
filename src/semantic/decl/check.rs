use crate::ast::TypeFeaturesKind;
use crate::ast::{Decl, DeclKind};
use crate::lexer::span::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::types::TypeId;

impl SemanticAnalyzer {
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
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::UnknownTypeInParameter {
                                        type_name: type_name.to_string(),
                                        param_name: param_name.to_string(),
                                    },
                                    function_decl.span,
                                )
                                .into(),
                            );
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
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::DuplicateParameter {
                                function: name.to_string(),
                                param: param_name.to_string(),
                            },
                            function_decl.span,
                        )
                        .into(),
                    );
                }
            }
            let expected_return = match return_type {
                Some(type_name) => match self.ctx.types.resolve(type_name) {
                    Some(id) => id,
                    None => {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::UnknownReturnType {
                                    function: name.to_string(),
                                    type_name: type_name.to_string(),
                                },
                                function_decl.span,
                            )
                            .into(),
                        );
                        self.ctx.types.resolve("Object").unwrap()
                    }
                },
                None => self.ctx.types.resolve("Object").unwrap(),
            };
            self.ctx.current_function_return = Some(expected_return);
            let body_type = self.check_expr(body);
            if !self.ctx.types.is_subtype_of(body_type, expected_return) {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::FunctionReturnTypeMismatch {
                            function: name.to_string(),
                            expected: self.ctx.types.get(body_type).name.clone(),
                            found: self.ctx.types.get(expected_return).name.clone(),
                        },
                        body.span,
                    )
                    .into(),
                );
            }
            self.ctx.current_function_return = None;
            self.ctx.pop_scope();
        }
    }

    fn check_type(&mut self, type_decl: &Decl) {
        if let DeclKind::Type {
            name,
            params,
            parent,
            features,
        } = &type_decl.node
        {
            let current_type_id = self
                .ctx
                .types
                .resolve(name)
                .expect("Type should have already been registered in the first pass");
            self.ctx.current_type = Some(current_type_id);
            self.ctx.push_scope();
            if let Some(param_list) = params {
                for (param_name, param_type_opt) in param_list {
                    let param_type_id = match param_type_opt {
                        Some(type_name) => match self.ctx.types.resolve(type_name) {
                            Some(id) => id,
                            None => {
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
                                self.ctx.types.resolve("Object").unwrap()
                            }
                        },
                        None => self.ctx.types.resolve("Object").unwrap(),
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
                }
            }
            if let Some(inherit_info) = parent {
                match self.ctx.types.resolve(&inherit_info.node.parent_name) {
                    Some(parent_type_id) => {
                        let expected_parent_params = self
                            .ctx
                            .types
                            .get_constructor_params(parent_type_id)
                            .to_vec();
                        let actual_args = inherit_info
                            .node
                            .args
                            .as_ref()
                            .map(|v| v.as_slice())
                            .unwrap_or_default();
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
                            let inferred_type_id = self.check_expr(arg_expr);
                            if i < expected_parent_params.len() {
                                let param_info = &expected_parent_params[i];
                                let expected_type_id = match param_info.ty {
                                    Some(id) => id,
                                    None => self.ctx.types.resolve("Object").unwrap(),
                                };
                                if !self
                                    .ctx
                                    .types
                                    .is_subtype_of(inferred_type_id, expected_type_id)
                                {
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
                                                    .get(inferred_type_id)
                                                    .name
                                                    .clone(),
                                            },
                                            arg_expr.span,
                                        )
                                        .into(),
                                    );
                                }
                            }
                        }
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
                    }
                }
            }
            self.ctx.declare(Symbol {
                name: "self".to_string(),
                kind: SymbolKind::Variable,
                ty: SymbolType::Variable(current_type_id),
                span: type_decl.span,
            });
            for feature in features {
                if let TypeFeaturesKind::Attribute {
                    name: attr_name,
                    type_name,
                    default,
                } = &feature.node
                {
                    let expected_attr_type = match type_name {
                        Some(t_name) => self.ctx.types.resolve(t_name).unwrap_or_else(|| {
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
                            self.ctx.types.resolve("Object").unwrap()
                        }),
                        None => self.ctx.types.resolve("Object").unwrap(),
                    };
                    if let Some(init_expr) = default {
                        let inferred_type = self.check_expr(init_expr);
                        if !self
                            .ctx
                            .types
                            .is_subtype_of(inferred_type, expected_attr_type)
                        {
                            self.diagnostics.push(
                                SemanticError::new(
                                    SemanticErrorKind::AttributeTypeMismatch {
                                        attribute: attr_name.to_string(),
                                        expected: self
                                            .ctx
                                            .types
                                            .get(expected_attr_type)
                                            .name
                                            .clone(),
                                        found: self.ctx.types.get(inferred_type).name.clone(),
                                    },
                                    type_decl.span,
                                )
                                .into(),
                            );
                        }
                    }
                    let attr_symbol = Symbol {
                        name: attr_name.clone(),
                        kind: SymbolKind::Attribute,
                        ty: SymbolType::Variable(expected_attr_type),
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
                }
            }
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
                    for (p_name, p_type_opt) in method_params {
                        let p_type_id = match p_type_opt {
                            Some(t_name) => match self.ctx.types.resolve(t_name) {
                                Some(id) => id,
                                None => {
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
                                    self.ctx.types.resolve("Object").unwrap()
                                }
                            },
                            None => self.ctx.types.resolve("Object").unwrap(),
                        };
                        current_method_param_types.push(p_type_id);
                        self.ctx.declare(Symbol {
                            name: p_name.clone(),
                            kind: SymbolKind::Parameter,
                            ty: SymbolType::Variable(p_type_id),
                            span: feature.span,
                        });
                    }
                    let expected_ret_id = match return_type {
                        Some(t_name) => match self.ctx.types.resolve(t_name) {
                            Some(id) => id,
                            None => {
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
                                self.ctx.types.resolve("Object").unwrap()
                            }
                        },
                        None => self.ctx.types.resolve("Object").unwrap(),
                    };
                    if let Some(parent_info) = parent {
                        if let Some(parent_id) =
                            self.ctx.types.resolve(&parent_info.node.parent_name)
                        {
                            if let Some(parent_method) =
                                self.ctx.types.get_method(parent_id, method_name).cloned()
                            {
                                self.validate_method_override(
                                    method_name,
                                    method_params,
                                    expected_ret_id,
                                    &parent_method.ty,
                                    type_decl.span,
                                );
                            }
                        }
                    }
                    self.ctx.current_function_return = Some(expected_ret_id);
                    self.ctx.current_method = Some(method_name.clone());
                    let body_type = self.check_expr(body);
                    if !self.ctx.types.is_subtype_of(body_type, expected_ret_id) {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::MethodReturnTypeMismatch {
                                    method: method_name.to_string(),
                                    expected: self.ctx.types.get(expected_ret_id).name.clone(),
                                    found: self.ctx.types.get(body_type).name.clone(),
                                },
                                body.span,
                            )
                            .into(),
                        );
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

    fn validate_method_override(
        &mut self,
        method_name: &str,
        current_params: &[(String, Option<String>)],
        current_ret: TypeId,
        parent_method_sig: &SymbolType,
        span: Span,
    ) {
        if let SymbolType::Function {
            params: parent_params,
            ret: parent_ret,
        } = parent_method_sig
        {
            if current_params.len() != parent_params.len() {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::InvalidOverrideArity {
                            method: method_name.to_string(),
                        },
                        span,
                    )
                    .into(),
                );
                return;
            }
            if current_ret != *parent_ret {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::InvalidOverrideReturnType {
                            method: method_name.to_string(),
                        },
                        span,
                    )
                    .into(),
                );
            }
            for (i, (_, p_type_opt)) in current_params.iter().enumerate() {
                let current_p_id = p_type_opt
                    .as_ref()
                    .and_then(|t| self.ctx.types.resolve(t))
                    .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                if current_p_id != parent_params[i] {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::InvalidOverrideParameterType {
                                method: method_name.to_string(),
                                index: i + 1,
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
