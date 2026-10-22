#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: String,
    pub text: String,
    pub base: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
    combined: String,
}

impl SourceMap {
    pub fn single(path: impl Into<String>, text: impl Into<String>) -> Self {
        let mut map = Self::default();
        map.add(path, text);
        map
    }

    pub fn add(&mut self, path: impl Into<String>, text: impl Into<String>) -> usize {
        let text = text.into();
        let base = self.combined.len();
        self.combined.push_str(&text);
        self.combined.push('\n');
        self.files.push(SourceFile {
            path: path.into(),
            text,
            base,
        });
        base
    }

    pub fn text(&self) -> &str {
        &self.combined
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    pub fn file_of(&self, offset: usize) -> &SourceFile {
        let index = self
            .files
            .partition_point(|file| file.base <= offset)
            .saturating_sub(1);

        &self.files[index.min(self.files.len() - 1)]
    }

    pub fn locate(&self, offset: usize) -> (&str, usize, usize) {
        if self.files.is_empty() {
            return ("", 1, 1);
        }

        let file = self.file_of(offset);
        let local = offset.saturating_sub(file.base);
        let (line, column) = line_col(&file.text, local);
        (&file.path, line, column)
    }

    pub fn position(&self, offset: usize) -> (usize, usize) {
        let (_, line, column) = self.locate(offset);
        (line, column)
    }

    pub fn path(&self, offset: usize) -> &str {
        self.locate(offset).0
    }

    pub fn entry_text(&self) -> &str {
        self.files.last().map(|file| file.text.as_str()).unwrap_or("")
    }

    pub fn entry_path(&self) -> &str {
        self.files.last().map(|file| file.path.as_str()).unwrap_or("")
    }
}

pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let column = source[line_start..offset.min(source.len())].chars().count() + 1;
    (line, column)
}
