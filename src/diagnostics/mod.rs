pub mod printer;

use std::cell::{Ref, RefCell};
use std::fmt::{Display, Formatter, Result};
use std::rc::Rc;

use crate::ast::token::{Span, TokenKind};
use crate::ast::types::{TypeId, TypeVarId};
use crate::ast::{ASTBinaryOperatorKind, ASTExprKind, ASTUnaryOperatorKind};
use crate::color::Color;

//
// ========================
// Diagnostic (final, immutable)
// ========================
//

#[derive(Debug)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub kind: DiagnosticKind,
    pub span: Span, // primary span
    pub segments: Vec<DiagnosticSegment>,
}

impl Diagnostic {
    pub fn header(&self) -> String {
        format!(
            "{}{}: {}{}",
            self.level.format(),
            Color::Bold,
            self.kind,
            Color::ResetAll
        )
    }
}

//
// ========================
// Diagnostic Builder
// ========================
//

pub struct DiagnosticBuilder {
    level: DiagnosticLevel,
    kind: DiagnosticKind,
    span: Span,
    segments: Vec<DiagnosticSegment>,
}

impl DiagnosticBuilder {
    pub fn error(kind: DiagnosticKind, span: Span) -> Self {
        Self::new(DiagnosticLevel::Error, kind, span)
    }

    pub fn warning(kind: DiagnosticKind, span: Span) -> Self {
        Self::new(DiagnosticLevel::Warning, kind, span)
    }

    pub fn tip(kind: DiagnosticKind, span: Span) -> Self {
        Self::new(DiagnosticLevel::Tip, kind, span)
    }

    fn new(level: DiagnosticLevel, kind: DiagnosticKind, span: Span) -> Self {
        Self {
            level,
            kind,
            span,
            segments: Vec::new(),
        }
    }

    // ----- segments -----

    pub fn label(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.segments.push(DiagnosticSegment::Label {
            span,
            message: msg.into(),
        });
        self
    }

    pub fn help(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.segments.push(DiagnosticSegment::Help {
            span,
            message: msg.into(),
        });
        self
    }

    pub fn note(mut self, msg: impl Into<String>) -> Self {
        self.segments.push(DiagnosticSegment::Note {
            message: msg.into(),
        });
        self
    }

    pub fn suggestion(
        mut self,
        span: Span,
        replacement: impl Into<String>,
        msg: impl Into<String>,
    ) -> Self {
        self.segments.push(DiagnosticSegment::Suggestion {
            span,
            replacement: replacement.into(),
            message: msg.into(),
        });
        self
    }

    pub fn build(self) -> Diagnostic {
        Diagnostic {
            level: self.level,
            kind: self.kind,
            span: self.span,
            segments: self.segments,
        }
    }
}

//
// ========================
// Diagnostic Segments
// ========================
//

#[derive(Debug)]
pub enum DiagnosticSegment {
    Label {
        span: Span,
        message: String,
    },
    Help {
        span: Span,
        message: String,
    },
    Note {
        message: String,
    },
    Suggestion {
        span: Span,
        replacement: String,
        message: String,
    },
}

//
// ========================
// Diagnostic Bag
// ========================
//

pub struct DiagnosticBag {
    diagnostics: RefCell<Vec<Diagnostic>>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self {
            diagnostics: RefCell::new(Vec::new()),
        }
    }

    pub fn push(&self, diag: Diagnostic) {
        self.diagnostics.borrow_mut().push(diag);
    }

    // pub fn extend(&self, diags: impl IntoIterator<Item = Diagnostic>) {
    //     self.diagnostics.borrow_mut().extend(diags);
    // }

    pub fn get(&self) -> Ref<'_, Vec<Diagnostic>> {
        self.diagnostics.borrow()
    }

    // pub fn is_empty(&self) -> bool {
    //     self.diagnostics.borrow().is_empty()
    // }
}

pub type DiagnosticBagCell = Rc<DiagnosticBag>;

//
// ========================
// Diagnostic Level
// ========================
//

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Tip,
}

impl DiagnosticLevel {
    fn format(&self) -> String {
        let (name, color) = match self {
            DiagnosticLevel::Error => ("error", "#FF686B"),
            DiagnosticLevel::Warning => ("warning", "#F5BD60"),
            DiagnosticLevel::Tip => ("tip", "#AED692"),
        };

        format!(
            "{}{}{}{}",
            Color::Bold,
            Color::FgHex(color),
            name,
            Color::ResetAll
        )
    }
}

//
// ========================
// Diagnostic Kind
// ========================
//

#[derive(Debug)]
pub enum DiagnosticKind {
    UnknownToken {
        given: char,
    },
    UnexpectedToken {
        given: TokenKind,
    },
    UnexpectedExpression {
        given: ASTExprKind,
    },
    ExpectedToken {
        expected: Vec<String>,
    },
    ExpectedExpression,
    MissingSemicolon,
    TypeMismatch {
        given: TypeId,
        expected: TypeId,
    },
    UnresolvedTypeVariable {
        id: TypeVarId,
    },
    UnknownIdentifier {
        identifier: String,
    },
    UnknownCharacter,
    InvalidCharacter,
    InvalidValue,
    InvalidGenericBase,
    InvalidBinaryOperator {
        op: ASTBinaryOperatorKind,
        left: TypeId,
        right: TypeId,
    },
    InvalidUnaryOperator {
        op: ASTUnaryOperatorKind,
        ty: TypeId,
    },
    AlreadyDefined {
        name: String,
    },
    ImmutableVariable,
    InvalidAssignmentTarget,
    InvalidCast {
        from: TypeId,
        to: TypeId,
    },
    OutOfBound,
    UnexpectedWhitespace,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DiagnosticKind::UnknownToken { given } => write!(f, "unknown token `{}`", given),
            DiagnosticKind::UnexpectedToken { given } => {
                write!(f, "unexpected token '{}'", given)
            }
            DiagnosticKind::UnexpectedExpression { given } => {
                write!(f, "unexpected expression '{}'", given)
            }
            DiagnosticKind::ExpectedToken { expected } => {
                write!(f, "expected {}", Self::join(expected))
            }
            DiagnosticKind::ExpectedExpression => {
                write!(f, "expected expression")
            }
            DiagnosticKind::MissingSemicolon => write!(f, "missing semicolon"),
            DiagnosticKind::TypeMismatch { given, expected } => {
                write!(
                    f,
                    "type mismatch: expected '{}', found '{}'",
                    expected, given
                )
            }
            DiagnosticKind::UnresolvedTypeVariable { id } => {
                write!(f, "unresolved type variable '{}'", id)
            }
            DiagnosticKind::UnknownIdentifier { identifier } => {
                write!(f, "unknown identifier '{}'", identifier)
            }
            DiagnosticKind::UnknownCharacter => write!(f, "unknown character"),
            DiagnosticKind::InvalidCharacter => write!(f, "invalid character"),
            DiagnosticKind::InvalidValue => write!(f, "invalid value"),
            DiagnosticKind::InvalidGenericBase => write!(f, "invalid generic base"),
            DiagnosticKind::InvalidUnaryOperator { op, ty } => {
                write!(f, "operator '{:?}' cannot be applied to type '{}'", op, ty)
            }
            DiagnosticKind::InvalidBinaryOperator { op, left, right } => write!(
                f,
                "operator '{:?}' cannot be applied to types '{}' and '{}'",
                op, left, right
            ),
            DiagnosticKind::AlreadyDefined { name } => {
                write!(f, "identifier '{}' already defined", name)
            }
            DiagnosticKind::ImmutableVariable => write!(f, "immutable variable"),
            DiagnosticKind::InvalidAssignmentTarget => write!(f, "invalid target for assignment"),
            DiagnosticKind::InvalidCast { from, to } => {
                write!(f, "invalid cast from '{}' to '{}'", from, to)
            }
            DiagnosticKind::OutOfBound => write!(f, "index out of bounds"),
            DiagnosticKind::UnexpectedWhitespace => {
                write!(f, "unexpected whitespace")
            }
        }
    }
}

impl DiagnosticKind {
    fn join(items: &[String]) -> String {
        match items.len() {
            0 => String::new(),
            1 => items[0].to_string(),
            2 => format!("{} or {}", items[0], items[1]),
            _ => {
                let (head, tail) = items.split_at(items.len() - 1);
                format!("{} or {}", head.join(", "), tail[0])
            }
        }
    }
}
