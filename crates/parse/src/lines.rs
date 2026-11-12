use model::Location;

/// Maps byte offsets to [`Location`]s. Line terminators follow ECMAScript: LF, CR, CRLF, LS, PS.
pub(crate) struct LineIndex<'s> {
    source: &'s str,
    starts: Vec<usize>,
}

impl<'s> LineIndex<'s> {
    pub(crate) fn new(source: &'s str) -> Self {
        let mut starts = vec![0];
        let mut chars = source.char_indices().peekable();
        while let Some((offset, c)) = chars.next() {
            match c {
                '\r' if chars.peek().is_some_and(|&(_, next)| next == '\n') => {}
                '\n' | '\r' | '\u{2028}' | '\u{2029}' => starts.push(offset + c.len_utf8()),
                _ => {}
            }
        }
        Self { source, starts }
    }

    pub(crate) fn location(&self, offset: u32) -> Location {
        let offset = (offset as usize).min(self.source.len());
        let line = self.starts.partition_point(|&start| start <= offset);
        let start = self.starts[line - 1];
        let column = self.source[start..offset].encode_utf16().count() + 1;
        Location {
            line: to_u32(line),
            column: to_u32(column),
        }
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::LineIndex;
    use model::Location;

    fn at(source: &str, offset: u32) -> (u32, u32) {
        let Location { line, column } = LineIndex::new(source).location(offset);
        (line, column)
    }

    #[test]
    fn first_character() {
        assert_eq!(at("abc", 0), (1, 1));
    }

    #[test]
    fn every_line_terminator() {
        let source = "a\nb\r\nc\rd\u{2028}e\u{2029}f";
        let lines: Vec<_> = ['a', 'b', 'c', 'd', 'e', 'f']
            .iter()
            .map(|&c| at(source, source.find(c).unwrap() as u32))
            .collect();
        assert_eq!(lines, [(1, 1), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1)]);
    }

    #[test]
    fn column_counts_utf16_code_units() {
        let source = "é😀x";
        assert_eq!(at(source, source.find('x').unwrap() as u32), (1, 4));
    }
}
