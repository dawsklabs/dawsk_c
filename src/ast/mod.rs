pub mod lexer;
pub mod parser;
// pub mod scope;
pub mod token;
// pub mod traits;
pub mod visitor;
// pub mod typechecker;
pub mod expander;
pub mod macros;
pub mod strings;

use std::fmt::{self, Display, Formatter};
use std::io;
use std::result::Result;

use crate::ast::strings::{StringId, StringPool};
use crate::ast::token::{NumSuffix, TokenKind};
use crate::ast::visitor::{ASTPrinter, ASTVisitor, Colors};
use crate::source::{Span, SpanSource};
use smallvec::SmallVec;

// use crate::ast::scope::NameId;

pub struct AST {
    pub items: Vec<ASTItem>,
}

impl AST {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_item(&mut self, item: ASTItem) {
        self.items.push(item);
    }

    pub fn visit<V: ASTVisitor>(&self, visitor: &mut V) -> Result<(), V::Error> {
        for item in &self.items {
            match item {
                ASTItem::Stmt(stmt) => visitor.visit_stmt(stmt)?,
                // ASTItem::Use(_) => {}
                // ASTItem::Mod(_) => {}
            }
        }

        Ok(())
    }

    pub fn visualize(&self, string_pool: &StringPool) {
        let mut output = io::stdout();
        let mut printer = ASTPrinter {
            indent: 0,
            out: &mut output,
            string_pool,
            color: Colors::new(),
        };
        let _ = self.visit(&mut printer);
    }
}

pub enum ASTItem {
    Stmt(ASTStmt),
    // Use(ASTUse),
    // Mod(ASTMod),
    // später: Fn, Struct, etc.
}

// #[derive(Debug, Clone)]
// pub struct ASTUse {
//     pub path: Box<[String]>,
//     pub alias: Option<String>,
// }

// #[derive(Debug, Clone)]
// pub struct ASTMod {
//     pub name: String,
// }

#[derive(Debug, Clone)]
pub enum ASTStmtKind {
    Expr(ASTExpr),
    // Return(ASTExpr),
    VarDec(ASTVarDecExpr),
    ConstDec(ASTConstDecExpr),
    StructDec(ASTStructDecExpr),
    TupleStructDec(ASTTupleStructDecExpr),
    UnitStructDec(ASTUnitStructDecExpr),
    EnumDec(ASTEnumDecExpr),
    TypeAliasDec(ASTTypeAliasDecExpr),
    FuncDec(ASTFuncDecExpr),
    MacroDec(ASTMacroDecExpr),
}

#[derive(Debug, Clone)]
pub struct ASTStmt {
    pub kind: ASTStmtKind,
}

impl ASTStmt {
    pub fn new(kind: ASTStmtKind) -> Self {
        Self { kind }
    }

    pub fn expr(expr: ASTExpr) -> Self {
        Self::new(ASTStmtKind::Expr(expr))
    }

    pub fn var_dec(
        ident: Ident,
        mut_: Mutability,
        ty: Option<ASTType>,
        initializer: ASTExpr,
    ) -> Self {
        Self::new(ASTStmtKind::VarDec(ASTVarDecExpr::new(
            ident,
            mut_,
            ty,
            initializer,
        )))
    }

    pub fn const_dec(ident: Ident, pub_: Publicity, ty: Option<ASTType>, expr: ASTExpr) -> Self {
        Self::new(ASTStmtKind::ConstDec(ASTConstDecExpr::new(
            ident, pub_, ty, expr,
        )))
    }

    pub fn struct_dec(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        fields: SmallVec<[ASTStructField; 4]>,
    ) -> Self {
        Self::new(ASTStmtKind::StructDec(ASTStructDecExpr::new(
            ident, pub_, generics, fields,
        )))
    }

    pub fn tuple_struct_dec(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        fields: SmallVec<[ASTTupleStructField; 4]>,
    ) -> Self {
        Self::new(ASTStmtKind::TupleStructDec(ASTTupleStructDecExpr::new(
            ident, pub_, generics, fields,
        )))
    }

    pub fn unit_struct_dec(ident: Ident, pub_: Publicity) -> Self {
        Self::new(ASTStmtKind::UnitStructDec(ASTUnitStructDecExpr::new(
            ident, pub_,
        )))
    }

    pub fn enum_dec(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        variants: SmallVec<[ASTEnumVariant; 4]>,
    ) -> Self {
        Self::new(ASTStmtKind::EnumDec(ASTEnumDecExpr::new(
            ident, pub_, generics, variants,
        )))
    }

    pub fn type_alias(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        ty: ASTType,
    ) -> Self {
        Self::new(ASTStmtKind::TypeAliasDec(ASTTypeAliasDecExpr::new(
            ident, pub_, generics, ty,
        )))
    }

    pub fn func_dec(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        params: SmallVec<[ASTFuncParam; 4]>,
        return_ty: Option<ASTType>,
        body: Box<ASTBlockExpr>,
    ) -> Self {
        Self::new(ASTStmtKind::FuncDec(ASTFuncDecExpr {
            ident,
            pub_,
            generics,
            params,
            return_ty,
            body,
        }))
    }

    pub fn macro_dec(
        ident: Ident,
        pub_: Publicity,
        rules: Vec<ASTMacroRule>,
        bracket_kind: MacroBracketKind,
    ) -> Self {
        Self::new(ASTStmtKind::MacroDec(ASTMacroDecExpr {
            ident,
            pub_,
            rules,
            bracket_kind,
        }))
    }
}

#[derive(Debug, Clone)]
pub enum ASTExprKind {
    Integer(u128, Option<NumSuffix>),
    Float(f64, Option<NumSuffix>),
    Byte(u8),            // b''
    Char(char),          // ''
    String(String),      // "", r""
    ByteString(Vec<u8>), // b"", br""
    Bool(bool),
    Unary(ASTUnaryExpr),
    Binary(ASTBinaryExpr),
    Cast(ASTCastExpr),
    Parenthesized(ASTParenExpr),
    Assignment(ASTAssignmentExpr),
    Variable(String),
    Block(Box<ASTBlockExpr>),
    FieldAccess(ASTFieldAccessExpr),
    MethodCall(ASTMethodCallExpr),
    Call(ASTCallExpr),
    Index(ASTIndexExpr),
    PathSegment(ASTPathSegmentExpr),
    MacroCall(ASTMacroCallExpr),
    Tuple(ASTTupleExpr),
    Unit,
    Error,
}

impl Display for ASTExprKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ASTExprKind::Integer(..) => write!(f, "Integer"),
            ASTExprKind::Float(..) => write!(f, "Float"),
            ASTExprKind::Byte(_) => write!(f, "Byte Char"),
            ASTExprKind::Char(_) => write!(f, "Char"),
            ASTExprKind::String(_) => write!(f, "String"),
            ASTExprKind::ByteString(_) => write!(f, "Byte String"),
            ASTExprKind::Bool(_) => write!(f, "Bool"),
            ASTExprKind::Parenthesized(_) => write!(f, "Parenthesized Expression"),
            ASTExprKind::Unary(_) => write!(f, "Unary Expression"),
            ASTExprKind::Binary(_) => write!(f, "Binary Expression"),
            ASTExprKind::Cast(_) => write!(f, "Cast Expression"),
            ASTExprKind::Assignment(_) => write!(f, "Assignment Expression"),
            ASTExprKind::Variable(..) => write!(f, "Variable"),
            ASTExprKind::Block(_) => write!(f, "Block Expression"),
            ASTExprKind::FieldAccess(_) => write!(f, "Field Access"),
            ASTExprKind::MethodCall(_) => write!(f, "Method Call"),
            ASTExprKind::Call(_) => write!(f, "Call"),
            ASTExprKind::Index(_) => write!(f, "Index"),
            ASTExprKind::PathSegment(_) => write!(f, "Path Segment"),
            ASTExprKind::MacroCall(_) => write!(f, "Macro Call"),
            ASTExprKind::Tuple(_) => write!(f, "Tuple"),
            ASTExprKind::Unit => write!(f, "Unit"),
            ASTExprKind::Error => write!(f, "ExprError"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTExpr {
    pub kind: ASTExprKind,
    pub span: Span,
}

impl ASTExpr {
    pub fn new(kind: ASTExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn int(value: u128, suffix: Option<NumSuffix>, span: Span) -> Self {
        Self::new(ASTExprKind::Integer(value, suffix), span)
    }

    pub fn float(value: f64, suffix: Option<NumSuffix>, span: Span) -> Self {
        Self::new(ASTExprKind::Float(value, suffix), span)
    }

    pub fn byte(value: u8, span: Span) -> Self {
        Self::new(ASTExprKind::Byte(value), span)
    }

    pub fn char(value: char, span: Span) -> Self {
        Self::new(ASTExprKind::Char(value), span)
    }

    pub fn string(value: String, span: Span) -> Self {
        Self::new(ASTExprKind::String(value), span)
    }

    pub fn byte_string(value: Vec<u8>, span: Span) -> Self {
        Self::new(ASTExprKind::ByteString(value), span)
    }

    pub fn bool(value: bool, span: Span) -> Self {
        Self::new(ASTExprKind::Bool(value), span)
    }

    pub fn binary(left: ASTExpr, right: ASTExpr, operator: ASTBinaryOperator, span: Span) -> Self {
        Self::new(
            ASTExprKind::Binary(ASTBinaryExpr::new(left, right, operator)),
            span,
        )
    }

    pub fn parenthesized(expr: ASTExpr, span: Span) -> Self {
        Self::new(ASTExprKind::Parenthesized(ASTParenExpr::new(expr)), span)
    }

    pub fn cast(expr: ASTExpr, target: ASTType, span: Span) -> Self {
        Self::new(ASTExprKind::Cast(ASTCastExpr::new(expr, target)), span)
    }

    pub fn assignment(
        target: ASTExpr,
        op: ASTBinaryOperatorKind,
        value: ASTExpr,
        span: Span,
    ) -> Self {
        Self::new(
            ASTExprKind::Assignment(ASTAssignmentExpr::new(target, op, value)),
            span,
        )
    }

    pub fn variable(name: String, span: Span) -> Self {
        Self::new(ASTExprKind::Variable(name), span)
    }

    pub fn unary(op: ASTUnaryOperator, expr: ASTExpr, span: Span) -> Self {
        Self::new(ASTExprKind::Unary(ASTUnaryExpr::new(op, expr)), span)
    }

    pub fn block(stmts: Box<[ASTStmt]>, tail: Option<ASTExpr>, span: Span) -> Self {
        Self::new(
            ASTExprKind::Block(Box::new(ASTBlockExpr::new(stmts, tail))),
            span,
        )
    }

    pub fn field_access(expr: ASTExpr, field: Ident, span: Span) -> Self {
        Self::new(
            ASTExprKind::FieldAccess(ASTFieldAccessExpr {
                expr: Box::new(expr),
                field,
            }),
            span,
        )
    }

    pub fn method_call(expr: ASTExpr, method: Ident, args: Vec<ASTExpr>, span: Span) -> Self {
        Self::new(
            ASTExprKind::MethodCall(ASTMethodCallExpr {
                expr: Box::new(expr),
                method,
                args,
            }),
            span,
        )
    }

    pub fn call(expr: ASTExpr, args: Vec<ASTExpr>, span: Span) -> Self {
        Self::new(
            ASTExprKind::Call(ASTCallExpr {
                expr: Box::new(expr),
                args,
            }),
            span,
        )
    }

    pub fn index(expr: ASTExpr, index: ASTExpr, span: Span) -> Self {
        Self::new(
            ASTExprKind::Index(ASTIndexExpr {
                expr: Box::new(expr),
                index: Box::new(index),
            }),
            span,
        )
    }

    pub fn path_segment(expr: ASTExpr, segment: Ident, span: Span) -> Self {
        Self::new(
            ASTExprKind::PathSegment(ASTPathSegmentExpr {
                expr: Box::new(expr),
                segment,
            }),
            span,
        )
    }

    pub fn macro_call(name: String, args: Vec<ASTExpr>, span: Span) -> Self {
        Self::new(
            ASTExprKind::MacroCall(ASTMacroCallExpr { name, args }),
            span,
        )
    }

    pub fn tuple(elems: Vec<ASTExpr>, span: Span) -> Self {
        Self::new(ASTExprKind::Tuple(ASTTupleExpr { elems }), span)
    }

    pub fn unit(span: Span) -> Self {
        Self::new(ASTExprKind::Unit, span)
    }

    pub fn error(file_id: usize) -> Self {
        Self::new(
            ASTExprKind::Error,
            Span::new(0, 0, file_id, SpanSource::Source),
        )
    }
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub id: StringId, // or Rc<str> for owned
    pub span: Span,
}

impl Ident {
    pub fn new(id: StringId, span: Span) -> Self {
        Self { id, span }
    }
}

#[derive(Debug, Clone)]
pub enum ASTType {
    Path(Vec<Ident>),
    Ref {
        mutable: Mutability,
        inner: Box<ASTType>,
    },
    Tuple(Vec<ASTType>),
    Generic {
        base: Box<ASTType>,
        args: Vec<ASTType>,
    },
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Publicity {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Mutable,
    Immutable,
}

#[derive(Debug, Clone)]
pub struct ASTBinaryExpr {
    pub left: Box<ASTExpr>,
    pub right: Box<ASTExpr>,
    pub operator: ASTBinaryOperator,
}

impl ASTBinaryExpr {
    pub fn new(left: ASTExpr, right: ASTExpr, operator: ASTBinaryOperator) -> Self {
        Self {
            left: Box::new(left),
            right: Box::new(right),
            operator,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ASTBinaryOperator {
    pub kind: ASTBinaryOperatorKind,
    pub span: Span,
}

impl ASTBinaryOperator {
    pub fn new(kind: ASTBinaryOperatorKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn prec(&self) -> u8 {
        match self.kind {
            // Arithmetic
            ASTBinaryOperatorKind::Multiply
            | ASTBinaryOperatorKind::Divide
            | ASTBinaryOperatorKind::Remainder => 3,
            ASTBinaryOperatorKind::Add | ASTBinaryOperatorKind::Subtract => 2,

            // Vergleich
            ASTBinaryOperatorKind::Less
            | ASTBinaryOperatorKind::Greater
            | ASTBinaryOperatorKind::LessEqual
            | ASTBinaryOperatorKind::GreaterEqual
            | ASTBinaryOperatorKind::Equal
            | ASTBinaryOperatorKind::NotEqual
            | ASTBinaryOperatorKind::LogicOr
            | ASTBinaryOperatorKind::LogicAnd => 1,

            // Assignment
            ASTBinaryOperatorKind::Assign
            | ASTBinaryOperatorKind::AddAssign
            | ASTBinaryOperatorKind::SubtractAssign
            | ASTBinaryOperatorKind::MultiplyAssign
            | ASTBinaryOperatorKind::DivideAssign => 0,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ASTBinaryOperatorKind {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,

    // Assignment
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,

    // Comparison
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=

    // Bitwise
    BitAnd, // &
    BitOr,  // |

    // Logical
    LogicOr,  // ||
    LogicAnd, // &&
    LogicXor, // ^

    LBitShift, // <<
    RBitShift, // >>
}

impl Display for ASTBinaryOperatorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ASTBinaryOperatorKind::Assign => write!(f, "Assign (=)"),
            ASTBinaryOperatorKind::Add => write!(f, "Add (+)"),
            ASTBinaryOperatorKind::AddAssign => write!(f, "Add (+) & Assign (=)"),
            ASTBinaryOperatorKind::Subtract => write!(f, "Subtract (-)"),
            ASTBinaryOperatorKind::SubtractAssign => write!(f, "Subtract (-) & Assign (=)"),
            ASTBinaryOperatorKind::Multiply => write!(f, "Multiply (*)"),
            ASTBinaryOperatorKind::MultiplyAssign => write!(f, "Multiply (*) & Assign (=)"),
            ASTBinaryOperatorKind::Divide => write!(f, "Divide (/)"),
            ASTBinaryOperatorKind::DivideAssign => write!(f, "Divide (/) & Assign (=)"),
            ASTBinaryOperatorKind::Remainder => write!(f, "Remainder (%)"),

            ASTBinaryOperatorKind::Equal => write!(f, "[?] Equal (==)"),
            ASTBinaryOperatorKind::NotEqual => write!(f, "[?] Not Equal (!=)"),
            ASTBinaryOperatorKind::Less => write!(f, "[?] Less (<)"),
            ASTBinaryOperatorKind::Greater => write!(f, "[?] Greater (>)"),
            ASTBinaryOperatorKind::LessEqual => write!(f, "[?] Less (<) || Equal (==)"),
            ASTBinaryOperatorKind::GreaterEqual => write!(f, "[?] Greater (>) || Equal (==)"),

            ASTBinaryOperatorKind::BitAnd => write!(f, "Bitwise And (&)"),
            ASTBinaryOperatorKind::BitOr => write!(f, "Bitwise Or (|)"),

            ASTBinaryOperatorKind::LogicAnd => write!(f, "Logical And (&&)"),
            ASTBinaryOperatorKind::LogicOr => write!(f, "Logical Or (||)"),
            ASTBinaryOperatorKind::LogicXor => write!(f, "Logical Xor (^)"),

            ASTBinaryOperatorKind::LBitShift => write!(f, "Left Bit Shift (<<)"),
            ASTBinaryOperatorKind::RBitShift => write!(f, "Right Bit Shift (>>)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTParenExpr {
    pub expr: Box<ASTExpr>,
}

impl ASTParenExpr {
    pub fn new(expr: ASTExpr) -> Self {
        Self {
            expr: Box::new(expr),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTVarDecExpr {
    pub ident: Ident,
    pub mut_: Mutability,
    pub ty: Option<ASTType>,
    pub initializer: ASTExpr,
}

impl ASTVarDecExpr {
    pub fn new(ident: Ident, mut_: Mutability, ty: Option<ASTType>, initializer: ASTExpr) -> Self {
        Self {
            ident,
            mut_,
            ty,
            initializer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTConstDecExpr {
    pub ident: Ident,
    pub pub_: Publicity,
    pub ty: Option<ASTType>,
    pub initializer: ASTExpr,
}

impl ASTConstDecExpr {
    pub fn new(ident: Ident, pub_: Publicity, ty: Option<ASTType>, initializer: ASTExpr) -> Self {
        Self {
            ident,
            pub_,
            ty,
            initializer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTStructDecExpr {
    pub ident: Ident,
    pub pub_: Publicity,
    pub generics: SmallVec<[ASTGenericParam; 2]>,
    pub fields: SmallVec<[ASTStructField; 4]>,
}

impl ASTStructDecExpr {
    pub fn new(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        fields: SmallVec<[ASTStructField; 4]>,
    ) -> Self {
        Self {
            ident,
            pub_,
            generics,
            fields,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTStructField {
    pub ident: Ident,
    pub pub_: Publicity,
    pub ty: ASTType,
}

#[derive(Debug, Clone)]
pub struct ASTTupleStructDecExpr {
    pub ident: Ident,
    pub pub_: Publicity,
    pub generics: SmallVec<[ASTGenericParam; 2]>,
    pub fields: SmallVec<[ASTTupleStructField; 4]>,
}

impl ASTTupleStructDecExpr {
    pub fn new(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        fields: SmallVec<[ASTTupleStructField; 4]>,
    ) -> Self {
        Self {
            ident,
            pub_,
            generics,
            fields,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTTupleStructField {
    pub pub_: Publicity,
    pub ty: ASTType,
}

#[derive(Debug, Clone)]
pub struct ASTUnitStructDecExpr {
    pub ident: Ident,
    pub pub_: Publicity,
}

impl ASTUnitStructDecExpr {
    pub fn new(ident: Ident, pub_: Publicity) -> Self {
        Self { ident, pub_ }
    }
}

#[derive(Debug, Clone)]
pub struct ASTEnumDecExpr {
    pub ident: Ident,
    pub pub_: Publicity,
    pub generics: SmallVec<[ASTGenericParam; 2]>,
    pub variants: SmallVec<[ASTEnumVariant; 4]>,
}

impl ASTEnumDecExpr {
    pub fn new(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        variants: SmallVec<[ASTEnumVariant; 4]>,
    ) -> Self {
        Self {
            ident,
            pub_,
            generics,
            variants,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTEnumVariant {
    pub ident: Ident,
    pub kind: ASTEnumVariantKind,
}

#[derive(Debug, Clone)]
pub enum ASTEnumVariantKind {
    Struct(SmallVec<[ASTStructField; 4]>),
    Tuple(SmallVec<[ASTTupleStructField; 4]>),
    Unit,
}

#[derive(Debug, Clone)]
pub struct ASTTypeAliasDecExpr {
    pub ident: Ident,
    pub pub_: Publicity,
    pub generics: SmallVec<[ASTGenericParam; 2]>,
    pub ty: ASTType,
}

impl ASTTypeAliasDecExpr {
    pub fn new(
        ident: Ident,
        pub_: Publicity,
        generics: SmallVec<[ASTGenericParam; 2]>,
        ty: ASTType,
    ) -> Self {
        Self {
            ident,
            pub_,
            generics,
            ty,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTMacroDecExpr {
    pub ident: Ident,
    pub pub_: Publicity,
    pub rules: Vec<ASTMacroRule>,
    pub bracket_kind: MacroBracketKind,
}

/// Eine einzelne Regel: (pattern) => { body }
#[derive(Debug, Clone)]
pub struct ASTMacroRule {
    pub pattern: Vec<MacroPatToken>,
    pub body: Vec<MacroBodyToken>,
}

/// Token im Pattern
#[derive(Debug, Clone)]
pub enum MacroPatToken {
    /// exact match
    Literal(TokenKind),
    /// $name:kind
    Capture {
        name: StringId,
        kind: CaptureKind,
        span: Span,
    },
    /// $(...sep)* or +
    Repetition {
        tokens: Vec<MacroPatToken>,
        separator: Option<TokenKind>,
        kind: RepKind,
    },
}

/// Token im Body
#[derive(Debug, Clone)]
pub enum MacroBodyToken {
    /// Direkt in Output
    Literal(TokenKind, Span),
    /// $name einsetzen
    Var(StringId, Span),
    /// $(...sep)* oder +
    Repetition {
        tokens: Vec<MacroBodyToken>,
        separator: Option<TokenKind>,
        kind: RepKind,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureKind {
    Expr,
    Ident,
    Ty,
    Literal,
    Stmt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepKind {
    ZeroOrMore, // *
    OneOrMore,  // +
}

#[derive(Debug, Clone, PartialEq)]
pub enum MacroBracketKind {
    Paren,  // ()
    Square, // []
    Curly,  // {}
}

#[derive(Debug, Clone)]
pub struct ASTAssignmentExpr {
    pub target: Box<ASTExpr>,
    pub op: ASTBinaryOperatorKind,
    pub value: Box<ASTExpr>,
}

impl ASTAssignmentExpr {
    pub fn new(target: ASTExpr, op: ASTBinaryOperatorKind, value: ASTExpr) -> Self {
        Self {
            target: Box::new(target),
            op,
            value: Box::new(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTCastExpr {
    pub expr: Box<ASTExpr>,
    pub target: ASTType,
}

impl ASTCastExpr {
    pub fn new(expr: ASTExpr, target: ASTType) -> Self {
        Self {
            expr: Box::new(expr),
            target,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTUnaryExpr {
    op: ASTUnaryOperator,
    expr: Box<ASTExpr>,
}

impl ASTUnaryExpr {
    pub fn new(op: ASTUnaryOperator, expr: ASTExpr) -> Self {
        Self {
            op,
            expr: Box::new(expr),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTUnaryOperator {
    pub kind: ASTUnaryOperatorKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTUnaryOperatorKind {
    Negate, // -
    Not,    // !

    PreIncrement,  // ++x
    PostIncrement, // x++

    PreDecrement,  // --x
    PostDecrement, // x--

    Ref,    // &
    RefMut, // &mut
    Deref,  // *
}

impl Display for ASTUnaryOperatorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ASTUnaryOperatorKind::Negate => write!(f, "Negate (-)"),
            ASTUnaryOperatorKind::Not => write!(f, "Logical Not (!)"),

            ASTUnaryOperatorKind::PreIncrement => write!(f, "Pre-Increment (++...)"),
            ASTUnaryOperatorKind::PostIncrement => write!(f, "Post-Increment (...++)"),

            ASTUnaryOperatorKind::PreDecrement => write!(f, "Pre-Decrement (--...)"),
            ASTUnaryOperatorKind::PostDecrement => write!(f, "Post-Decrement (...--)"),

            ASTUnaryOperatorKind::Ref => write!(f, "Reference (&)"),
            ASTUnaryOperatorKind::RefMut => write!(f, "Mutable Reference (&mut)"),
            ASTUnaryOperatorKind::Deref => write!(f, "Dereference (*)"),
        }
    }
}

impl ASTUnaryOperator {
    pub fn new(kind: ASTUnaryOperatorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub struct ASTBlockExpr {
    pub stmts: Box<[ASTStmt]>,
    pub tail: Option<ASTExpr>,
}

impl ASTBlockExpr {
    pub fn new(stmts: Box<[ASTStmt]>, tail: Option<ASTExpr>) -> Self {
        Self { stmts, tail }
    }
}

#[derive(Debug, Clone)]
pub struct ASTGenericParam {
    pub name: Ident,              // Name: T, U, ...
    pub default: Option<ASTType>, // e.g. = Inst
}

impl ASTGenericParam {
    pub fn new(name: Ident, default: Option<ASTType>) -> Self {
        Self { name, default }
    }
}

#[derive(Debug, Clone)]
pub struct ASTFieldAccessExpr {
    pub expr: Box<ASTExpr>,
    pub field: Ident,
}

#[derive(Debug, Clone)]
pub struct ASTMethodCallExpr {
    pub expr: Box<ASTExpr>,
    pub method: Ident,
    pub args: Vec<ASTExpr>,
}

#[derive(Debug, Clone)]
pub struct ASTCallExpr {
    pub expr: Box<ASTExpr>,
    pub args: Vec<ASTExpr>,
}

#[derive(Debug, Clone)]
pub struct ASTIndexExpr {
    pub expr: Box<ASTExpr>,
    pub index: Box<ASTExpr>,
}

#[derive(Debug, Clone)]
pub struct ASTPathSegmentExpr {
    pub expr: Box<ASTExpr>,
    pub segment: Ident,
}

#[derive(Debug, Clone)]
pub struct ASTMacroCallExpr {
    pub name: String,
    pub args: Vec<ASTExpr>,
}

#[derive(Debug, Clone)]
pub struct ASTTupleExpr {
    pub elems: Vec<ASTExpr>,
}

#[derive(Debug, Clone)]
pub struct ASTFuncDecExpr {
    pub ident: Ident,
    pub pub_: Publicity,
    pub generics: SmallVec<[ASTGenericParam; 2]>,
    pub params: SmallVec<[ASTFuncParam; 4]>,
    pub return_ty: Option<ASTType>,
    pub body: Box<ASTBlockExpr>,
}

#[derive(Debug, Clone)]
pub enum ASTFuncParam {
    Receiver {
        mutable: Mutability, // &inst vs &mut inst
        span: Span,
    },
    Named {
        ident: Ident,
        mutable: Mutability,
        ty: ASTType,
        span: Span,
    },
}
