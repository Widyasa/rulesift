//! Line/column helpers shared by all parsers.

/// Index of line start offsets for a text, mapping byte offsets to 1-based
/// line and character columns and back.
#[derive(Debug, Clone)]
pub struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    /// Build the index. Lines end at `\n` (a preceding `\r` belongs to the line).
    pub fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        Self { text, starts }
    }

    /// Number of lines.
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// Byte offset of 1-based `line` and 1-based byte column `col`, clamped
    /// to the text and moved back to a char boundary.
    pub fn offset(&self, line: usize, byte_col: usize) -> usize {
        let line = line.clamp(1, self.starts.len());
        let off = self.starts[line - 1] + byte_col.saturating_sub(1);
        self.floor_boundary(off.min(self.text.len()))
    }

    /// Byte offset of the start of 1-based `line`.
    pub fn line_start(&self, line: usize) -> usize {
        self.starts[line.clamp(1, self.starts.len()) - 1]
    }

    /// 1-based (line, character column) of a byte offset.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let offset = self.floor_boundary(offset.min(self.text.len()));
        let line = match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let col = self.text[self.starts[line]..offset].chars().count() + 1;
        (line + 1, col)
    }

    /// Largest char boundary ≤ `off`.
    pub fn floor_boundary(&self, mut off: usize) -> usize {
        while off > 0 && !self.text.is_char_boundary(off) {
            off -= 1;
        }
        off
    }

    /// Smallest char boundary ≥ `off` (clamped to the text length).
    pub fn ceil_boundary(&self, mut off: usize) -> usize {
        off = off.min(self.text.len());
        while off < self.text.len() && !self.text.is_char_boundary(off) {
            off += 1;
        }
        off
    }
}

/// 1-based line of the byte `offset` inside `text` (counts `\n`).
pub fn line_of(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_roundtrip() {
        let t = "ab\ncé\r\nx";
        let li = LineIndex::new(t);
        assert_eq!(li.line_count(), 3);
        assert_eq!(li.position(0), (1, 1));
        assert_eq!(li.position(3), (2, 1));
        assert_eq!(li.position(4), (2, 2));
        assert_eq!(li.position(6), (2, 3)); // after é (2 bytes)
        assert_eq!(li.offset(3, 1), 8);
        assert_eq!(li.offset(2, 3), 4); // inside é -> floor
        assert_eq!(line_of(t, 8), 3);
    }
}
