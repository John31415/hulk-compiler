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

impl SemanticAnalyzer {
    pub fn check_expr(&mut self, expr: &Expr) -> TypeId {
        match &expr.node {
            ExprKind::Literal(lit) => self.check_literal(lit),
            ExprKind::Variable(name) => self.check_variable(name, expr.span),
            ExprKind::Block(expressions) => self.check_block(expressions, expr.span),
            ExprKind::Unary { op, expr } => self.check_unary(op, expr),
            ExprKind::Binary {
                left_expr,
                op,
                right_expr,
            } => self.check_binary(left_expr, op, right_expr),
            ExprKind::Assign { target, value } => self.check_assign(target, value),
            ExprKind::Let {
                name,
                type_name,
                value,
                body,
            } => self.check_let(name, type_name, value, body, expr.span),
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.check_if(condition, then_branch, else_branch, expr.span),
            ExprKind::While { condition, body } => self.check_while(condition, body),
            ExprKind::For {
                var,
                iterable,
                body,
            } => self.check_for(var, iterable, body, expr.span),
            ExprKind::New { type_name, args } => self.check_new(type_name, args, expr.span),
            ExprKind::PropertyAccess { obj, property } => {
                self.check_property_access(obj, property, expr.span)
            }
            ExprKind::MethodCall { obj, method, args } => {
                self.check_method_call(obj, method, args, expr.span)
            }
            ExprKind::Is { expr, type_name } => self.check_is(expr, type_name, expr.span),
            ExprKind::As { expr, type_name } => self.check_as(expr, type_name, expr.span),
            ExprKind::Call { name, args } if name == "base" => {
                self.check_base_call(name, args, expr.span)
            }
            ExprKind::Call { name, args } => self.check_call(name, args, expr.span),
        }
    }

    pub fn resolve_builtin(&mut self, type_name: &str) -> TypeId {
        self.ctx
            .types
            .resolve(type_name)
            .unwrap_or_else(|| self.ctx.types.resolve("Object").unwrap())
    }
}
