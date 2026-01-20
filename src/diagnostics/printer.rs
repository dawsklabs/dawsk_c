use crate::color::Color;
use crate::diagnostics::{DiagnosticBag, DiagnosticSegment};
use crate::text::{file, Source};

pub struct Printer<'a> {
    source: &'a Source,
    diagnostics: &'a mut DiagnosticBag,
}

impl<'a> Printer<'a> {
    pub fn new(source: &'a Source, diagnostics: &'a mut DiagnosticBag) -> Self {
        Self {
            source,
            diagnostics,
        }
    }

    pub fn stringify(&self) -> String {
        let mut out = String::new();
        let file = file::get();

        const COLOR_BLUE: Color = Color::FgHex("#7FB0FF");
        const COLOR_RED: Color = Color::FgHex("#FF686B");
        const COLOR_GREEN: Color = Color::FgHex("#AED692");

        out.push('\n');

        let diagnostics = self.diagnostics.get();

        for (i, diag) in diagnostics.iter().enumerate() {
            // ============================================================
            // Header
            // ============================================================
            out.push_str(&diag.header());
            out.push('\n');

            // ============================================================
            // Position
            // ============================================================
            let (line_no, _) = self.source.line_col(diag.span.start);
            let line_bytes = self.source.line_at(diag.span.start);
            let line_str = String::from_utf8_lossy(line_bytes);

            let line_len = line_str.len();
            let col = self.source.col(diag.span.start).min(line_len);
            let span_end = (col + diag.span.len()).min(line_len);
            let caret_len = span_end.saturating_sub(col).max(1);

            let line_no_width = line_no.to_string().len();

            // --> file:line:col
            out.push_str(&format!(
                "{:>width$}{}{}-->{} {}:{}:{}\n",
                "",
                Color::Bold,
                COLOR_BLUE,
                Color::ResetAll,
                file,
                line_no,
                span_end + 1,
                width = line_no_width
            ));

            // |
            out.push_str(&format!(
                "{:>width$} {}{}|{}\n",
                "",
                Color::Bold,
                COLOR_BLUE,
                Color::ResetAll,
                width = line_no_width
            ));

            // line | code
            out.push_str(&format!(
                "{}{}{:>width$} |{} {}\n",
                Color::Bold,
                COLOR_BLUE,
                line_no,
                Color::ResetAll,
                line_str,
                width = line_no_width
            ));

            // caret
            out.push_str(&format!(
                "{:>width$} {}{}|{} ",
                "",
                Color::Bold,
                COLOR_BLUE,
                Color::ResetAll,
                width = line_no_width
            ));
            out.push_str(&" ".repeat(col));
            out.push_str(&format!(
                "{}{}{}{}",
                Color::Bold,
                COLOR_RED,
                "^".repeat(caret_len),
                Color::ResetAll
            ));
            out.push('\n');

            // ============================================================
            // Segments (labels, help, notes, suggestions)
            // ============================================================
            for segment in &diag.segments {
                match segment {
                    DiagnosticSegment::Label { span, message } => {
                        self.render_inline_span(&mut out, span, message, COLOR_BLUE, line_no_width);
                    }

                    DiagnosticSegment::Help { span, message } => {
                        let primary_line = self.source.line_col(diag.span.start).0;
                        let help_line = self.source.line_col(span.start).0;

                        if primary_line == help_line {
                            self.render_inline_span(
                                &mut out,
                                span,
                                &format!(
                                    "{}{}help{}: {}",
                                    Color::Bold,
                                    COLOR_GREEN,
                                    Color::ResetAll,
                                    message
                                ),
                                COLOR_GREEN,
                                line_no_width,
                            );
                        } else {
                            out.push('\n');
                            self.render_secondary_block(
                                &mut out,
                                span,
                                &format!(
                                    "{}{}help{}: {}",
                                    Color::Bold,
                                    COLOR_GREEN,
                                    Color::ResetAll,
                                    message
                                ),
                                COLOR_GREEN,
                            );
                        }
                    }

                    DiagnosticSegment::Suggestion { span, message, .. } => {
                        self.render_inline_span(
                            &mut out,
                            span,
                            &format!("suggestion: {}", message),
                            COLOR_GREEN,
                            line_no_width,
                        );
                    }

                    DiagnosticSegment::Note { message } => {
                        out.push_str(&format!(
                            "{:>width$} {}{}= note:{} {}\n",
                            "",
                            Color::Bold,
                            COLOR_BLUE,
                            Color::ResetAll,
                            message,
                            width = line_no_width
                        ));
                    }
                }
            }

            if i + 1 < diagnostics.len() {
                out.push('\n');
            }
        }

        out
    }

    // ============================================================
    // Helper: render secondary span
    // ============================================================
    fn render_inline_span(
        &self,
        out: &mut String,
        span: &crate::ast::token::Span,
        message: &str,
        color: Color,
        line_no_width: usize,
    ) {
        let line_bytes = self.source.line_at(span.start);
        let line_str = String::from_utf8_lossy(line_bytes);

        let line_len = line_str.len();
        let col = self.source.col(span.start).min(line_len);
        let span_end = (col + span.len()).min(line_len);
        let caret_len = span_end.saturating_sub(col).max(1);

        out.push_str(&format!(
            "{:>width$} {}{}|{} ",
            "",
            Color::Bold,
            Color::FgHex("#7FB0FF"),
            Color::ResetAll,
            width = line_no_width
        ));
        out.push_str(&" ".repeat(col));
        out.push_str(&format!(
            "{}{}{} {}{}\n",
            Color::Bold,
            color,
            "-".repeat(caret_len),
            message,
            Color::ResetAll
        ));
    }

    fn render_secondary_block(
        &self,
        out: &mut String,
        span: &crate::ast::token::Span,
        message: &str,
        color: Color,
    ) {
        let file = file::get();
        let (line_no, _) = self.source.line_col(span.start);
        let line_bytes = self.source.line_at(span.start);
        let line_str = String::from_utf8_lossy(line_bytes);

        let line_len = line_str.len();
        let col = self.source.col(span.start).min(line_len);
        let span_end = (col + span.len()).min(line_len);
        let caret_len = span_end.saturating_sub(col).max(1);

        let width = line_no.to_string().len();

        // --> file:line:col
        out.push_str(&format!(
            "{:>width$}{}{}-->{} {}:{}:{}\n",
            "",
            Color::Bold,
            Color::FgHex("#7FB0FF"),
            Color::ResetAll,
            file,
            line_no,
            span_end + 1,
            width = width
        ));

        // |
        out.push_str(&format!(
            "{:>width$} {}{}|{}\n",
            "",
            Color::Bold,
            Color::FgHex("#7FB0FF"),
            Color::ResetAll,
            width = width
        ));

        // code line
        out.push_str(&format!(
            "{}{}{:>width$} |{} {}\n",
            Color::Bold,
            Color::FgHex("#7FB0FF"),
            line_no,
            Color::ResetAll,
            line_str,
            width = width
        ));

        // caret + message
        out.push_str(&format!(
            "{:>width$} {}{}|{} ",
            "",
            Color::Bold,
            Color::FgHex("#7FB0FF"),
            Color::ResetAll,
            width = width
        ));
        out.push_str(&" ".repeat(col));
        out.push_str(&format!(
            "{}{}{}{} {}\n",
            Color::Bold,
            color,
            "^".repeat(caret_len),
            Color::ResetAll,
            message
        ));
    }
}
