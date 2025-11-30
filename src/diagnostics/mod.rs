pub mod printer;

use std::fmt::{Display, Formatter, Result};
use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::ast::token::{Span, TokenKind};
use crate::ast::ASTExprKind;
use crate::color::Color;

#[derive(Debug)]
pub struct Diagnostic {
    type_: DiagnosticType,
    span: Span,
}

impl Diagnostic {
    pub fn new(type_: DiagnosticType, span: Span) -> Self {
        Self {
            type_,
            span,
        }
    }

    fn make(&self) -> String {
        format!("{}{}: {}{}", self.format_type(), Color::Bold, self.kind(), Color::Reset)
    }

    fn format_type(&self) -> String {
        let color = match &self.type_ {
            DiagnosticType::Error(_) => "#FF686B",
            DiagnosticType::Warning(_) => "#F5BD60",
            DiagnosticType::Tip(_) => "#AED692",
        };

        format!("{}{}{}{}", Color::Bold, Color::FgHex(color), self.type_, Color::Reset)
    }

    fn kind(&self) -> &DiagnosticKind {
        match &self.type_ {
            DiagnosticType::Error(kind) | DiagnosticType::Warning(kind) | DiagnosticType::Tip(kind) => &kind,
        }
    }
}

pub struct DiagnosticBag {
    diagnostics: RefCell<Vec<Diagnostic>>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self {
            diagnostics: RefCell::new(vec![]),
        }
    }

    pub fn add(&self, diag: Diagnostic) {
        self.diagnostics.borrow_mut().push(diag);
    }

    pub fn get(&self) -> Ref<'_, Vec<Diagnostic>> {
        self.diagnostics.borrow()
    }
}

pub type DiagnosticBagCell = Rc<DiagnosticBag>;

#[derive(Debug)]
pub enum DiagnosticType {
    Error(DiagnosticKind),
    Warning(DiagnosticKind),
    Tip(DiagnosticKind),
}

impl Display for DiagnosticType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #![allow(unreachable_patterns)]
        match self {
            DiagnosticType::Error(_) => write!(f, "error"),
            DiagnosticType::Warning(_) => write!(f, "warning"),
            DiagnosticType::Tip(_) => write!(f, "tip"),
            _ => write!(f, "..."),
        }
    }
}

#[derive(Debug)]
pub enum DiagnosticKind {
    UnknownToken { given: char },
    UnexpectedToken { given: TokenKind },
    UnexpectedExpression { given: ASTExprKind },
    ExpectedToken { expected: Vec<String> },
    ExpectedExpression { expected: Vec<String> },
    MissingSemicolon,
    // TypeMismatch { given: DataType, expected: DataType },
    UnknownIdentifier { identifier: String },
    UnknownCharacter { character: char },
    DuplicateVariable { name: String },
    OutOfBound,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #![allow(unreachable_patterns)]
        match self {
            DiagnosticKind::UnexpectedToken { given } => write!(f, "Unexpected token: {}", given),
            DiagnosticKind::UnexpectedExpression { given } => write!(f, "Unexpected expression: {}", given),
            DiagnosticKind::ExpectedToken { expected } => write!(f, "Expected token of kind: {}", Self::join_vec(expected)),
            DiagnosticKind::ExpectedExpression { expected } => write!(f, "Expected expression of kind: {}", Self::join_vec(expected)),
            DiagnosticKind::MissingSemicolon => write!(f, "Missing semicolon"),
            // DiagnosticKind::TypeMismatch { .. } => write!(f, "Type mismatch"),
            DiagnosticKind::UnknownIdentifier { identifier } => write!(f, "Unknown identifier: {}", identifier),
            DiagnosticKind::UnknownCharacter { character } => write!(f, "Unknown character: {}", character.to_string()),
            DiagnosticKind::DuplicateVariable { name } => write!(f, "Duplicate variable: {}", name),
            DiagnosticKind::OutOfBound => write!(f, "Out of bound"),
            _ => write!(f, "{}", self),
        }
    }
}

impl DiagnosticKind {
    fn join_vec(items: &[String]) -> String {
        match items.len() {
            0 => String::new(),
            1 => items[0].clone(),
            2 => format!("{} or {}", items[0], items[1]),
            _ => {
                let all_but_last = &items[..items.len()-1];
                let last = &items[items.len()-1];
                format!("{} or {}", all_but_last.join(", "), last)
            }
        }
    }
}