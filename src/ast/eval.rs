use super::{
    AST, ASTStmt, ASTStmtKind, ASTExpr, ASTExprKind, ASTBinaryExpr, ASTBinaryOperatorKind, ASTDecExpr, ASTAssignmentExpr, ASTUnaryExpr, ASTUnaryOperatorKind,
};
use super::token::TokenKind;
use super::scope::{ScopeStack, Symbol};
use super::types::TypeKind;

use std::{ fmt::{ Display, Formatter, Error }, result::Result };
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
    Uninitialized,
    Error,
    // later: Array(Vec<Value>), Object(HashMap<String, Value>), etc.
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(v) => write!(f, "{}", v),
            Value::Char(v) => write!(f, "'{}'", v),
            Value::String(v) => write!(f, "\"{}\"", v),
            Value::Uninitialized => write!(f, "UNINITIALIZED(None)"),
            Value::Error => write!(f, "!ERROR!"),
        }
    }
}

// #[derive(Debug, Clone, PartialEq)]
// pub enum ValueType {
//     I8, I16, I32, I64,
//     F32, F64,
// }

pub type EvalResult = Result<Value, String>;

pub struct Evaluator {
    last_val: Option<Value>,
    pub scope: ScopeStack,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            last_val: None,
            scope: ScopeStack::new(),
        }
    }

    pub fn eval_ast(&mut self, ast: &AST) -> EvalResult {
        let mut last = Value::Int(0);
        for stmt in &ast.stmts {
            last = self.eval_stmt(stmt)?;
        }
        Ok(last)
    }

    fn eval_stmt(&mut self, stmt: &ASTStmt) -> EvalResult {
        #![warn(unreachable_patterns)]
        match &stmt.kind {
            ASTStmtKind::Expr(expr) => self.eval_expr(expr),
            ASTStmtKind::Dec(dec) => self.eval_dec(dec),
            _ => todo!()
        }
    }

    fn eval_expr(&mut self, expr: &ASTExpr) -> EvalResult {
        match &expr.kind {
            ASTExprKind::Integer(v) => Ok(Value::Int(*v)),
            ASTExprKind::Float(v) => Ok(Value::Float(*v)),
            ASTExprKind::Char(v) => Ok(Value::Char(*v)),
            ASTExprKind::String(v) => Ok(Value::String((*v).clone())),
            ASTExprKind::Bool(v) => Ok(Value::Bool(*v)),
            ASTExprKind::Parenthesized(p) => self.eval_expr(&p.expr),
            ASTExprKind::Binary(b) => self.eval_binary_expr(b),
            ASTExprKind::Assignment(a) => self.eval_assignment_expr(a),
            ASTExprKind::Variable(name) => {
                if let Some(sym) = self.scope.lookup(name) {
                    // wir brauchen den aktuellen Wert, evtl. separat speichern
                    Ok(self.scope.lookup_value(name).unwrap()) // vorausgesetzt Symbol hat Value-Feld, sonst ScopeStack muss Werte halten
                } else {
                    Err(format!("Undefined variable '{}'", name))
                }
            }
            ASTExprKind::Unary(u) => self.eval_unary_expr(u.clone()),
            ASTExprKind::Error => Ok(Value::Error),
        }
    }

    fn eval_dec(&mut self, dec: &ASTDecExpr) -> EvalResult {
        let name = match &dec.identifier.kind {
            TokenKind::Identifier(s) => s.clone(),
            _ => return Err("Declaration identifier is not an identifier".to_string()),
        };

        let value = self.eval_expr(&dec.initializer)?; // hier wird der Wert berechnet

        let symbol = Symbol {
            name: name.clone(),
            type_: dec.type_.clone().unwrap_or(TypeKind::Custom("<unknown>".into())),
            mut_: dec.mut_,
            vis: true,
        };

        self.scope.define(symbol, value.clone())?; // Scope filling

        Ok(value)
    }

    fn eval_assignment_expr(&mut self, a: &ASTAssignmentExpr) -> EvalResult {
        let rhs = self.eval_expr(&a.value)?;

        // Lookup Symbol und Wert
        let sym = self.scope.lookup(&a.name)
            .ok_or_else(|| format!("Undefined variable '{}'", a.name))?;

        if !sym.mut_ {
            return Err(format!("Cannot assign to immutable variable '{}'", a.name));
        }

        let name = sym.name.clone();

        let new_value = match a.op {
            ASTBinaryOperatorKind::Assign => rhs.clone(),
            ASTBinaryOperatorKind::AddAssign => self.apply_binary_op(&self.scope.lookup_value(&name).unwrap(), &rhs, &ASTBinaryOperatorKind::Add)?,
            ASTBinaryOperatorKind::SubtractAssign => self.apply_binary_op(&self.scope.lookup_value(&name).unwrap(), &rhs, &ASTBinaryOperatorKind::Subtract)?,
            ASTBinaryOperatorKind::MultiplyAssign => self.apply_binary_op(&self.scope.lookup_value(&name).unwrap(), &rhs, &ASTBinaryOperatorKind::Multiply)?,
            ASTBinaryOperatorKind::DivideAssign => self.apply_binary_op(&self.scope.lookup_value(&name).unwrap(), &rhs, &ASTBinaryOperatorKind::Divide)?,
            _ => return Err(format!("Unsupported assignment operator: {:?}", a.op)),
        };

        // neuen Wert speichern
        // sym.value = new_value.clone();
        self.scope.assign(&a.name, new_value.clone())?;

        Ok(new_value)
    }

    fn eval_binary_expr(&mut self, bin: &ASTBinaryExpr) -> EvalResult {
        let left = self.eval_expr(&bin.left)?;
        let right = self.eval_expr(&bin.right)?;
        self.apply_binary_op(&left, &right, &bin.operator.kind)
    }

    fn apply_binary_op(&self, a: &Value, b: &Value, op: &ASTBinaryOperatorKind) -> EvalResult {
        use ASTBinaryOperatorKind::*;

        match op {
            // --- Numeric Operators ---
            Add => match (a, b) {
                (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x + y)),
                (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x + y)),
                // automatic Upcasting:
                (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 + *y)),
                (Value::Float(x), Value::Int(y)) => Ok(Value::Float(*x + *y as f64)),
                // String-Concatenation
                (Value::String(x), Value::String(y)) => Ok(Value::String(format!("{}{}", x, y))),
                (Value::String(x), Value::Char(y)) => Ok(Value::String(format!("{}{}", x, y))),
                _ => Err(format!("Cannot apply '+' to {:?} and {:?}", a, b)),
            },

            Subtract => match (a, b) {
                (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x - y)),
                (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x - y)),
                // automatic Upcasting:
                (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 - *y)),
                (Value::Float(x), Value::Int(y)) => Ok(Value::Float(*x - *y as f64)),
                _ => Err(format!("Cannot apply '-' to {:?} and {:?}", a, b)),
            },

            Multiply => match (a, b) {
                (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x * y)),
                (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x * y)),
                // automatic Upcasting:
                (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 * *y)),
                (Value::Float(x), Value::Int(y)) => Ok(Value::Float(*x * *y as f64)),
                _ => Err(format!("Cannot apply '*' to {:?} and {:?}", a, b)),
            },

            Divide => match (a, b) {
                (Value::Int(_), Value::Int(0)) => Err("Division by zero".into()),
                (Value::Float(_), Value::Float(y)) if *y == 0.0 => Err("Division by zero".into()),
                (Value::Int(x), Value::Int(y)) => Ok(Value::Float(*x as f64 / *y as f64)),
                (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x / y)),
                // automatic Upcasting:
                (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 / *y)),
                (Value::Float(x), Value::Int(y)) => Ok(Value::Float(*x / *y as f64)),
                _ => Err(format!("Cannot apply '/' to {:?} and {:?}", a, b)),
            },

            Modulus => match (a, b) {
                (Value::Int(x), Value::Int(y)) => if *y == 0 { Err("Modulo by zero".into()) } else if *x == 0 { Ok(Value::Int(0)) } else { Ok(Value::Int(x % y)) }
                _ => Err(format!("Cannot apply '%' to {:?} and {:?}", a, b)),
            },

            Assign | AddAssign | SubtractAssign | MultiplyAssign | DivideAssign => {
                return Err(format!("Assignment operator {:?} not allowed in binary expression", op));
            }
        }
    }

    fn eval_unary_expr(&mut self, u: ASTUnaryExpr) -> EvalResult {
        // Zuerst das innere Expr evaluieren
        let val = self.eval_expr(&u.expr)?;

        match u.op.kind {
            ASTUnaryOperatorKind::Negate => match val {
                Value::Int(x) => Ok(Value::Int(-x)),
                Value::Float(x) => Ok(Value::Float(-x)),
                _ => Err(format!("Cannot negate {:?}", val)),
            },
            ASTUnaryOperatorKind::Not => match val {
                Value::Bool(x) => Ok(Value::Bool(!x)),
                _ => Err(format!("Cannot apply '!' to {:?}", val)),
            },
        }
    }
}
