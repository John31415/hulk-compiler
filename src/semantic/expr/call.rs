use crate::ast::Expr;
use crate::lexer::span::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::SymbolType;
use crate::semantic::{analyzer::SemanticAnalyzer, types::TypeId};

impl SemanticAnalyzer {
    pub fn check_call(&mut self, name: &str, args: &Vec<Expr>, span: Span) -> TypeId {
        let function_sig = if let Some(symbol) = self.ctx.lookup(name) {
            match &symbol.ty {
                SymbolType::Function { params, ret } => Some((params.clone(), *ret)),
                SymbolType::Variable(_) => {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::NotAFunction {
                                name: name.to_string(),
                            },
                            span,
                        )
                        .into(),
                    );
                    return self.ctx.types.resolve("Object").unwrap();
                }
                SymbolType::Unknown => {
                    return self.ctx.types.resolve("Object").unwrap();
                }
            }
        } else {
            None
        };
        let (param_types, return_type) = match function_sig {
            Some(sig) => sig,
            None => {
                if let Some(current_type_id) = self.ctx.current_type {
                    if let Some((params, ret)) = self.ctx.types.lookup_method(current_type_id, name)
                    {
                        (params, ret)
                    } else {
                        self.diagnostics.push(
                            SemanticError::new(
                                SemanticErrorKind::UndefinedFunction {
                                    name: name.to_string(),
                                },
                                span,
                            )
                            .into(),
                        );
                        return self.ctx.types.resolve("Object").unwrap();
                    }
                } else {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::UndefinedFunction {
                                name: name.to_string(),
                            },
                            span,
                        )
                        .into(),
                    );
                    return self.ctx.types.resolve("Object").unwrap();
                }
            }
        };
        if args.len() != param_types.len() {
            self.diagnostics.push(
                SemanticError::new(
                    SemanticErrorKind::InvalidFunctionArity {
                        name: name.to_string(),
                        expected: param_types.len(),
                        found: args.len(),
                    },
                    span,
                )
                .into(),
            );
        }
        for (i, arg) in args.iter().enumerate() {
            let arg_type = self.check_expr(arg);
            if i < param_types.len() {
                let expected_type = param_types[i];
                if !self.ctx.types.is_subtype_of(arg_type, expected_type) {
                    self.diagnostics.push(
                        SemanticError::new(
                            SemanticErrorKind::FunctionArgumentTypeMismatch {
                                name: name.to_string(),
                                index: i + 1,
                                expected: self.ctx.types.get(expected_type).name.clone(),
                                found: self.ctx.types.get(arg_type).name.clone(),
                            },
                            arg.span,
                        )
                        .into(),
                    );
                    return self.ctx.types.resolve("Object").unwrap();
                }
            }
        }
        return_type
    }

    pub fn check_base_call(&mut self, _name: &str, args: &Vec<Expr>, span: Span) -> TypeId {
        let current_type_id = match self.ctx.current_type {
            Some(id) => id,
            None => {
                self.diagnostics
                    .push(SemanticError::new(SemanticErrorKind::InvalidBaseUsage, span).into());
                return self.ctx.types.resolve("Object").unwrap();
            }
        };
        let current_method_name = match &self.ctx.current_method {
            Some(m) => m,
            None => {
                self.diagnostics
                    .push(SemanticError::new(SemanticErrorKind::InvalidBaseUsage, span).into());
                return self.ctx.types.resolve("Object").unwrap();
            }
        };
        if !args.is_empty() {
            self.diagnostics
                .push(SemanticError::new(SemanticErrorKind::BaseTakesNoArguments, span).into());
        }
        match self.find_closest_ancestor_method(current_type_id, current_method_name) {
            Some(ancestor_return_type) => ancestor_return_type,
            None => {
                self.diagnostics.push(
                    SemanticError::new(
                        SemanticErrorKind::UndefinedBaseMethod {
                            type_name: self.ctx.types.get(current_type_id).name.clone(),
                            method: current_method_name.to_string(),
                        },
                        span,
                    )
                    .into(),
                );
                self.ctx.types.resolve("Object").unwrap()
            }
        }
    }

    fn find_closest_ancestor_method(&self, type_id: TypeId, method_name: &str) -> Option<TypeId> {
        let mut current_id = type_id;
        while let Some(parent_id) = self.ctx.types.get_parent(current_id) {
            if let Some(return_type_id) = self
                .ctx
                .types
                .get_method_return_type(parent_id, method_name)
            {
                return Some(return_type_id);
            }
            current_id = parent_id;
        }
        None
    }
}
