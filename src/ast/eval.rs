use super::{
    AST, ASTStmt, ASTStmtKind, ASTExpr, ASTExprKind, ASTBinaryExpr, ASTBinaryOperatorKind,
};
use std::{ fmt::{ Display, Formatter, Error }, result::Result };
use fxhash::FxHashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
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
    vars: FxHashMap<String, Value>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self { last_val: None, vars: FxHashMap::default() }
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
            ASTStmtKind::Dec(_expr) => todo!(),
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
            ASTExprKind::Error => Ok(Value::Error),
        }
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
                (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x / y)),
                (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x / y)),
                // automatic Upcasting:
                (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 / *y)),
                (Value::Float(x), Value::Int(y)) => Ok(Value::Float(*x / *y as f64)),
                _ => Err(format!("Cannot apply '/' to {:?} and {:?}", a, b)),
            },

            Modulus => match (a, b) {
                (Value::Int(x), Value::Int(y)) => if *y == 0 { Err("Modulo by zero".into()) } else { Ok(Value::Int(x % y)) }
                _ => Err(format!("Cannot apply '%' to {:?} and {:?}", a, b)),
            },
        }
    }
}
