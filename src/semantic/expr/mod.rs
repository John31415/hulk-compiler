pub mod assign;
pub mod binary;
pub mod block;
pub mod call;
pub mod control_flow;
pub mod let_expr;
pub mod new;
pub mod postfix;
pub mod primary;
pub mod unary;
use super::{analyzer::SemanticAnalyzer, types::TypeId};
use crate::ast::{Expr, ExprKind};
use crate::semantic::hir::TypedExpr;

impl SemanticAnalyzer {
    pub fn analyze_expr(&mut self, expression: &Expr) -> TypedExpr {
        match &expression.node {
            ExprKind::Literal(lit) => self.analyze_literal(lit, expression.span),
            ExprKind::Variable(name) => self.analyze_variable(name, expression.span),
            ExprKind::Block(expressions) => self.analyze_block(expressions, expression.span),
            ExprKind::Unary { op, expr } => self.analyze_unary(op, expr, expression.span),
            ExprKind::Binary {
                left_expr,
                op,
                right_expr,
            } => self.analyze_binary(left_expr, op, right_expr, expression.span),
            ExprKind::Assign { target, value } => {
                self.analyze_assign(target, value, expression.span)
            }
            ExprKind::Let {
                name,
                type_name,
                value,
                body,
            } => self.analyze_let(name, type_name, value, body, expression.span),
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.analyze_if(condition, then_branch, else_branch, expression.span),
            ExprKind::While { condition, body } => {
                self.analyze_while(condition, body, expression.span)
            }
            ExprKind::For {
                var,
                iterable,
                body,
            } => self.analyze_for(var, iterable, body, expression.span),
            ExprKind::New { type_name, args } => self.analyze_new(type_name, args, expression.span),
            ExprKind::PropertyAccess { obj, property } => {
                self.analyze_property_access(obj, property, expression.span)
            }
            ExprKind::MethodCall { obj, method, args } => {
                self.analyze_method_call(obj, method, args, expression.span)
            }
            ExprKind::Is { expr, type_name } => self.analyze_is(expr, type_name, expression.span),
            ExprKind::As { expr, type_name } => self.analyze_as(expr, type_name, expression.span),
            ExprKind::Call { name, args } if name == "base" => {
                self.analyze_base_call(name, args, expression.span)
            }
            ExprKind::Call { name, args } => self.analyze_call(name, args, expression.span),
        }
    }

    pub fn resolve_builtin(&mut self, type_name: &str) -> TypeId {
        self.ctx
            .types
            .resolve(type_name)
            .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap())
    }
}
