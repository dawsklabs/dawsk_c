use crate::diagnostics::{Diagnostic, DiagnosticBag};
use crate::text::Source;

pub struct Printer<'a> {
    source: &'a Source,
    diagnostics: &'a [Diagnostic],
}

impl<'a> Printer<'a> {
    pub fn new(source: &'a Source, diagnostics: &'a [Diagnostic]) -> Self {
        Self { source, diagnostics }
    }

    pub fn stringify(&self) -> String {
        let mut result = String::new();
        for diag in self.diagnostics {
            result.push_str(&diag.make());
            result.push('\n');
            // result.push_str(&self.source.text[diag.pos.span.start..diag.pos.span.end]);
            // result.push('\n');
            result.push_str(&"-".repeat(diag.pos.span.len()));
            result.push('\n');
        }
        result
    }
}