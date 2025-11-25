pub mod file;

pub struct Source {
    pub text: Vec<u8>,
}

impl Source {
    pub fn new(t: Vec<u8>) -> Self {
        Self { text: t }
    }

    // get the line start
    pub fn line_start(&self, pos: usize) -> usize {
        let bytes = &self.text;
        let end = pos.min(bytes.len());

        match memchr::memrchr(b'\n', &bytes[..end]) {
            Some(i) => i + 1,
            None => 0,
        }
    }

    // gives back the line and column number at the position
    pub fn line_col(&self, pos: usize) -> (usize, usize) {
        let end = pos.min(self.text.len());

        // Zeile = 1 + Anzahl '\n' vor pos
        let line = 1 + memchr::memchr_iter(b'\n', &self.text[..end]).count();

        // Column = pos - line_start + 1
        // line_start ist die Position nach dem letzten '\n'
        let line_start = match memchr::memrchr(b'\n', &self.text[..end]) {
            Some(i) => i + 1,
            None => 0,
        };

        (line, pos - line_start)
    }

    // gives back the line number at the position
    pub fn line(&self, pos: usize) -> usize {
        self.line_col(pos).0
    }

    // gives back the column number at the position
    pub fn col(&self, pos: usize) -> usize {
        self.line_col(pos).1
    }

    // gives back the full line at the position
    pub fn line_at(&self, pos: usize) -> &[u8] {
        let text = &self.text;

        let end = pos.min(text.len());

        // find start of current line
        let line_start = memchr::memrchr(b'\n', &text[..end])
            .map(|i| i + 1)
            .unwrap_or(0);

        // find end of current line
        let line_end = memchr::memchr(b'\n', &text[end..])
            .map(|i| end + i)
            .unwrap_or(text.len());

        &text[line_start..line_end]
    }
}