use crate::ast::TypeFeaturesKind;
use crate::ast::{Decl, DeclKind};
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::{Symbol, SymbolKind, SymbolType};
use crate::semantic::types::ConstructorParam;

impl SemanticAnalyzer {
    pub fn register_signatures(&mut self, decls: &[Decl]) {
        for decl in decls {
            match &decl.node {
                DeclKind::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let mut param_types: Vec<Option<crate::semantic::types::TypeId>> = Vec::new();
                    let mut any_param_generic = false;
                    for (_, param_type_opt) in params {
                        match param_type_opt {
                            Some(type_name) => {
                                let resolved = self.ctx.types.resolve(type_name);
                                if resolved.is_none() {
                                    any_param_generic = true;
                                }
                                param_types.push(resolved);
                            }
                            None => {
                                any_param_generic = true;
                                param_types.push(None);
                            }
                        }
                    }
                    let ret_resolved = return_type.as_ref().and_then(|t| self.ctx.types.resolve(t));
                    let ret_is_generic =
                        return_type.is_some() && ret_resolved.is_none() || return_type.is_none();
                    let new_ty = if any_param_generic || ret_is_generic {
                        self.ctx.register_generic_decl(name.clone(), decl.clone());
                        SymbolType::GenericFunction {
                            param_types,
                            ret_type: ret_resolved,
                        }
                    } else {
                        let concrete_params: Vec<_> =
                            param_types.into_iter().map(|t| t.unwrap()).collect();
                        SymbolType::Function {
                            params: concrete_params,
                            ret: ret_resolved.unwrap(),
                        }
                    };
                    if !self.ctx.update_symbol_type(name, new_ty.clone()) {
                        self.ctx.declare(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Function,
                            ty: new_ty,
                            span: decl.span,
                        });
                    }
                }
                DeclKind::Type {
                    name,
                    params,
                    parent,
                    features,
                } => {
                    let current_type_id =
                        self.ctx.types.resolve(name).expect(
                            "Type should have already been registered in collect_declarations",
                        );
                    if let Some(inherit_info) = parent {
                        if let Some(parent_type_id) =
                            self.ctx.types.resolve(&inherit_info.node.parent_name)
                        {
                            let parent_name = &inherit_info.node.parent_name;
                            if parent_name == "Number"
                                || parent_name == "String"
                                || parent_name == "Boolean"
                            {
                                self.diagnostics.push(
                                    SemanticError::new(
                                        SemanticErrorKind::InvalidInheritanceFromPrimitive {
                                            child: name.to_string(),
                                            parent: parent_name.to_string(),
                                        },
                                        inherit_info.span,
                                    )
                                    .into(),
                                );
                                self.ctx.types.set_parent(current_type_id, None);
                            } else {
                                self.ctx
                                    .types
                                    .set_parent(current_type_id, Some(parent_type_id));
                            }
                        }
                    }
                    let constructor_params = params.as_ref().map(|param_list| {
                        let mut v = Vec::new();
                        for (p_name, p_type_opt) in param_list {
                            let p_type =
                                p_type_opt.as_ref().and_then(|t| self.ctx.types.resolve(t));
                            v.push(ConstructorParam {
                                name: p_name.clone(),
                                ty: p_type,
                            });
                        }
                        v
                    });
                    self.ctx
                        .types
                        .set_declared_constructor_params(current_type_id, constructor_params);
                    for feature in features {
                        if let TypeFeaturesKind::Method {
                            name: method_name,
                            params: method_params,
                            return_type,
                            ..
                        } = &feature.node
                        {
                            let mut p_types = Vec::new();
                            for (_, p_type_opt) in method_params {
                                let p_id = p_type_opt
                                    .as_ref()
                                    .and_then(|t| self.ctx.types.resolve(t))
                                    .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                                p_types.push(p_id);
                            }
                            let r_id = return_type
                                .as_ref()
                                .and_then(|t| self.ctx.types.resolve(t))
                                .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap());
                            let method_symbol = Symbol {
                                name: method_name.clone(),
                                kind: SymbolKind::Function,
                                ty: SymbolType::Function {
                                    params: p_types,
                                    ret: r_id,
                                },
                                span: feature.span,
                            };
                            self.ctx.types.insert_method(current_type_id, method_symbol);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

    #[test]
    fn semantic_unit_test_register_inherits_primitive() {
        let source = r#"
type A inherits Number {
    a = 1;
}

42;
        "#;
        let program = parse_program(source);
        let mut analyzer = SemanticAnalyzer::new();
        let _ = analyzer.analyze_program(program);
        assert_eq!(analyzer.diagnostics.len(), 1);
        assert_eq!(
            analyzer.diagnostics[0].kind,
            SemanticErrorKind::InvalidInheritanceFromPrimitive {
                child: "A".to_string(),
                parent: "Number".to_string()
            }
        );
    }
}
