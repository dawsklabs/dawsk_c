use crate::diagnostics::{DiagnosticBagCell};
use crate::text::{Source, file};
use crate::color::Color;
use std::cmp;

pub struct Printer<'a> {
    source: &'a Source,
    diagnostics: DiagnosticBagCell,
}

const PREFIX_LEN: usize = 16;

impl<'a> Printer<'a> {
    pub fn new(source: &'a Source, diagnostics: DiagnosticBagCell) -> Self {
        Self { source, diagnostics }
    }

    pub fn stringify(&self) -> String {
        let mut out = String::new();
        let file = file::get();

        let blue = Color::FgHex("7FB0FF".to_string());
        let red  = Color::FgHex("FF686B".to_string());

        out.push_str("\n");

        for (i, diag) in self.diagnostics.get().iter().enumerate() {
            let msg = diag.make();

            // ---------------------------------------------------------------------
            // Position
            // ---------------------------------------------------------------------
            let (line_no, col_no) = self.source.line_col(diag.span.start);
            let line_bytes = self.source.line_at(diag.span.start);
            let line_str = String::from_utf8_lossy(line_bytes);

            let line_len = line_str.len();
            let col = self.source.col(diag.span.start).min(line_len);
            let span_end = (col + diag.span.len()).min(line_len);
            let caret_len = span_end.saturating_sub(col).max(1);

            // ---------------------------------------------------------------------
            // rustc-header
            // ---------------------------------------------------------------------
            out.push_str(&format!(
                "{}\n",
                msg
            ));

            let line_no_width = line_no.to_string().len();

            out.push_str(&format!(
                "{:>width$}{}{}-->{} {}:{}:{}\n",
                "",
                Color::Bold,
                blue,
                Color::Reset,
                file,
                line_no,
                span_end + 1,
                width = line_no_width
            ));

            // Pipe über der Codezeile
            out.push_str(&format!("{:>width$} {}{}|{}\n", "", Color::Bold, blue, Color::Reset, width = line_no_width));

            // Codezeile
            out.push_str(&format!("{}{}{:>width$} |{} {}\n", Color::Bold, blue, line_no, Color::Reset, line_str, width = line_no_width));

            // Caret-Zeile
            out.push_str(&format!("{:>width$} {}{}|{} ", "", Color::Bold, blue, Color::Reset, width = line_no_width));
            out.push_str(&" ".repeat(col)); // Padding bis Fehler-Spalte
            out.push_str(&format!(
                "{}{}{}{}",
                Color::Bold,
                red,
                "^".repeat(caret_len),
                Color::Reset
            ));
            // print empty line when not last diagnostic
            if i != self.diagnostics.get().len() - 1 {
                out.push_str("\n\n");
            }
        }

        out
    }
}