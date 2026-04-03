use std::{
    convert::Infallible,
    fmt::{Debug, Display},
    ops::Range, path::PathBuf, sync::Arc,
};

use unicode_width::UnicodeWidthStr;

use crate::reports::{Cache, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanSource {
    Source,
    Macro(MacroId),
}

pub struct MacroData {
    pub call_site: Span,
    pub def_site: Span,
}

pub type FileId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file_id: FileId,
    pub start: usize,
    pub end: usize,
    pub source: SpanSource,
}

impl Span {
    pub fn new(start: usize, end: usize, file_id: usize, source: SpanSource) -> Self {
        Self {
            file_id,
            start,
            end,
            source,
        }
    }

    pub fn merge(a: Span, b: Span) -> Span {
        debug_assert_eq!(a.file_id, b.file_id);
        Span {
            file_id: a.file_id,
            start: a.start.min(b.start),
            end: b.end.max(a.end),
            source: if a.source == b.source { a.source } else { SpanSource::Source },
        }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

impl crate::reports::Span for Span {
    type SourceId = usize;

    fn source(&self) -> &Self::SourceId {
        &self.file_id
    }
    fn start(&self) -> usize {
        self.start
    }
    fn end(&self) -> usize {
        self.end
    }
}

impl From<Span> for (usize, Range<usize>) {
    fn from(span: Span) -> Self {
        (span.file_id, span.start..span.end)
    }
}

pub struct SpanInfo<'a> {
    pub file: &'a PathBuf,
    pub line: usize,
    pub column: usize,
    pub line_text: &'a str,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SourceFile {
    pub entry: PathBuf,
    pub source: Source<Arc<str>>,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(entry: PathBuf) -> Self {
        let text: Arc<str> = std::fs::read_to_string(&entry)
            .unwrap()
            .into();  // String -> Arc<str>, keine Kopie
        
        let mut line_starts = vec![0];
        for (i, c) in text.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }

        Self {
            entry,
            source: Source::from(Arc::clone(&text)), // nur Pointer-Klon
            line_starts,
        }
    }

    pub fn line_col(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.source.len);

        let line_idx = match self.line_starts.binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };

        let line_start = self.line_starts[line_idx];

        let slice = &self.source.text[line_start..pos];
        let col = UnicodeWidthStr::width(slice);

        (line_idx + 1, col + 1)
    }

    pub fn get_line(&self, line: usize) -> &str {
        let start = self.line_starts[line - 1];

        let end = if line < self.line_starts.len() {
            self.line_starts[line] - 1
        } else {
            self.source.len
        };

        &self.source.text[start..end]
    }
}

pub struct SourceMap {
    files: Vec<SourceFile>,
    macros: Vec<MacroData>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            macros: Vec::new(),
        }
    }

    /// Registriert eine Makro-Expansion und gibt eine ID zurück
    pub fn register_macro(&mut self, call_site: Span, def_site: Span) -> SpanSource {
        let id = self.macros.len();
        self.macros.push(MacroData { call_site, def_site });
        SpanSource::Macro(MacroId(id))
    }

    pub fn add_file(&mut self, entry: PathBuf) -> usize {
        let id = self.files.len();
        self.files.push(SourceFile::new(entry));
        id
    }

    pub fn file(&self, id: usize) -> &SourceFile {
        &self.files[id]
    }

    /// Holt die Makro-Daten anhand der ID
    pub fn get_macro_data(&self, id: MacroId) -> &MacroData {
        &self.macros[id.0]
    }

    pub fn span_info(&self, span: Span) -> SpanInfo<'_> {
        let file = &self.files[span.file_id];
        let (line, col) = file.line_col(span.start);

        SpanInfo {
            file: &file.entry,
            line,
            column: col,
            line_text: file.get_line(line),
        }
    }
}

impl Cache<usize> for SourceMap {
    type Storage = Arc<str>;

    #[allow(refining_impl_trait)]
    fn fetch(&mut self, id: &usize) -> Result<&Source<Self::Storage>, Infallible> {
        Ok(&self.files[*id].source)
    }

    fn display<'a>(&self, id: &'a usize) -> Option<impl Display + 'a> {
        Some(self.files[*id].entry.to_string_lossy().into_owned())
    }
}
