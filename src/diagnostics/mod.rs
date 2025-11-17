pub mod printer;

use std::fmt::{Display, Formatter, Result};
use std::{rc::Rc, cell::RefCell};

use crate::ast::token::{Position, TokenKind};

pub struct Diagnostic {
    type_: DiagnosticType,
    pos: Position,
}

impl Diagnostic {
    pub fn new(type_: DiagnosticType, pos: Position) -> Self {
        Self {
            type_,
            pos,
        }
    }

    fn make(&self) -> String {
        let mut result: String = format!("{} {}\n", self.type_, self.kind()).to_string();
        result.push_str(&format!("--> {}:{}:{}\n", self.pos.file, self.pos.line, self.pos.span.start));
        result
    }

    fn kind(&self) -> &DiagnosticKind {
        match &self.type_ {
            DiagnosticType::Error(kind) | DiagnosticType::Warning(kind) => &kind,
        }
    }
}

pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self {
            diagnostics: vec![],
        }
    }

    pub fn add(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn render(&self) -> String {
        let mut result = String::new();
        for diagnostic in &self.diagnostics {
            result.push_str(&(diagnostic.make()));
            result.push('\n'); // Add a newline between each diagnostic for readability.
        }
        result
    }

    pub fn get(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

pub type DiagnosticBagCell = Rc<RefCell<DiagnosticBag>>;

pub enum DiagnosticType {
    Error(DiagnosticKind),
    Warning(DiagnosticKind),
}

impl Display for DiagnosticType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #![allow(unreachable_patterns)]
        match self {
            DiagnosticType::Error(_) => write!(f, "ERROR"),
            DiagnosticType::Warning(_) => write!(f, "WARNING"),
            _ => write!(f, "DIAGNOSTIC"),
        }
    }
}

pub enum DiagnosticKind {
    UnknownToken { given: char },
    UnexpectedToken { given: TokenKind, expected: Vec<TokenKind> },
    MissingSemicolon,
    // TypeMismatch { given: DataType, expected: DataType },
    UnknownIdentifier { identifier: String },
    OutOfBound,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #![allow(unreachable_patterns)]
        match self {
            DiagnosticKind::UnknownToken { given } => write!(f, "Unexpected token '{}'", given),
            DiagnosticKind::UnexpectedToken { .. } => write!(f, "Unexpected token"),
            DiagnosticKind::MissingSemicolon => write!(f, "Missing semicolon"),
            // DiagnosticKind::TypeMismatch { .. } => write!(f, "Type mismatch"),
            DiagnosticKind::UnknownIdentifier { identifier } => write!(f, "Unknown identifier: {}", identifier),
            DiagnosticKind::OutOfBound => write!(f, "Out of bound"),
            _ => write!(f, "{}", self),
        }
    }
}