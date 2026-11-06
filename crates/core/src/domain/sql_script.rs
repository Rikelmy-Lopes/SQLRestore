//! Reads a PostgreSQL plain-text backup as a stream of executable statements.
//!
//! The reader understands quoted strings, dollar-quoted bodies, comments and
//! `COPY ... FROM stdin` data blocks. It also ignores psql meta-commands that
//! cannot be executed by a database client. Statements are returned one at a
//! time, so a large backup is never loaded into memory as a whole.

use std::io::{self, Read, Write};

const BUFFER_SIZE: usize = 64 * 1024;
const MAX_DOLLAR_TAG_LEN: usize = 1024;

/// One executable statement of a SQL script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptStatement {
    /// Line in the source file where the statement starts.
    pub start_line: usize,
    /// Byte offset just after the statement in the source file.
    pub end_offset: u64,
    /// SQL statement, including the terminating semicolon when present.
    pub sql: String,
    /// True when the statement is `COPY ... FROM stdin`.
    pub copy: bool,
}

/// One executable piece of a SQL script, with its COPY data collected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptChunk {
    /// Line in the source file where the chunk starts.
    pub start_line: usize,
    /// Byte offset just after the chunk in the source file.
    pub end_offset: u64,
    /// SQL statement, including the terminating semicolon when present.
    pub sql: String,
    /// Data for a `COPY ... FROM stdin` statement.
    pub copy_data: Option<String>,
}

impl ScriptChunk {
    /// True when the chunk is a `COPY ... FROM stdin` statement.
    pub fn is_copy(&self) -> bool {
        self.copy_data.is_some()
    }
}

/// Result of reading a `COPY ... FROM stdin` data block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyDataOutcome {
    Completed,
    Cancelled,
}

/// Streams statements from a SQL script without loading the whole file.
pub struct SqlScriptReader<R: Read> {
    reader: R,
    buffer: Vec<u8>,
    position: usize,
    filled: usize,
    eof: bool,
    line: usize,
    offset: u64,
    at_line_start: bool,
    state: State,
    statement: Vec<u8>,
    statement_line: usize,
    has_content: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Normal,
    LineComment,
    BlockComment { depth: usize },
    SingleQuote,
    DoubleQuote,
    DollarQuote { tag: Vec<u8> },
}

impl<R: Read> SqlScriptReader<R> {
    /// Creates a reader over the given source.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: vec![0; BUFFER_SIZE],
            position: 0,
            filled: 0,
            eof: false,
            line: 1,
            offset: 0,
            at_line_start: true,
            state: State::Normal,
            statement: Vec::new(),
            statement_line: 1,
            has_content: false,
        }
    }

    /// Bytes consumed from the source so far.
    pub fn position(&self) -> u64 {
        self.offset
    }

    /// Returns the next SQL statement, or `None` at the end of the file.
    pub fn next_statement(&mut self) -> Result<Option<ScriptStatement>, io::Error> {
        loop {
            let Some(byte) = self.peek(0)? else {
                return self.finish();
            };

            let state = std::mem::replace(&mut self.state, State::Normal);
            match state {
                State::Normal => {
                    if !self.has_content {
                        match byte {
                            b if b.is_ascii_whitespace() => {
                                self.consume(1);
                                continue;
                            }
                            b'-' if self.peek(1)? == Some(b'-') => {
                                self.state = State::LineComment;
                                self.consume(2);
                                continue;
                            }
                            b'/' if self.peek(1)? == Some(b'*') => {
                                self.state = State::BlockComment { depth: 1 };
                                self.consume(2);
                                continue;
                            }
                            b'\\' if self.at_line_start => {
                                self.state = State::LineComment;
                                self.consume(1);
                                continue;
                            }
                            b';' => {
                                self.consume(1);
                                continue;
                            }
                            _ => {
                                self.has_content = true;
                                self.statement_line = self.line;
                            }
                        }
                    }

                    match byte {
                        b'\'' => {
                            self.append(1)?;
                            self.state = State::SingleQuote;
                        }
                        b'"' => {
                            self.append(1)?;
                            self.state = State::DoubleQuote;
                        }
                        b'-' if self.peek(1)? == Some(b'-') => {
                            self.append(2)?;
                            self.state = State::LineComment;
                        }
                        b'/' if self.peek(1)? == Some(b'*') => {
                            self.append(2)?;
                            self.state = State::BlockComment { depth: 1 };
                        }
                        b'$' => {
                            if let Some(tag) = self.dollar_tag()? {
                                let length = tag.len();
                                self.append(length)?;
                                self.state = State::DollarQuote { tag };
                            } else {
                                self.append(1)?;
                            }
                        }
                        b';' => {
                            self.append(1)?;
                            return self.take_statement().map(Some);
                        }
                        _ => self.append(1)?,
                    }
                }
                State::LineComment => {
                    self.append_if_content(1)?;
                    if byte != b'\n' {
                        self.state = State::LineComment;
                    }
                }
                State::BlockComment { depth } => {
                    if byte == b'/' && self.peek(1)? == Some(b'*') {
                        self.append_if_content(2)?;
                        self.state = State::BlockComment { depth: depth + 1 };
                    } else if byte == b'*' && self.peek(1)? == Some(b'/') {
                        self.append_if_content(2)?;
                        if depth > 1 {
                            self.state = State::BlockComment { depth: depth - 1 };
                        }
                    } else {
                        self.append_if_content(1)?;
                        self.state = State::BlockComment { depth };
                    }
                }
                State::SingleQuote => {
                    if byte == b'\'' {
                        if self.peek(1)? == Some(b'\'') {
                            self.append_if_content(2)?;
                            self.state = State::SingleQuote;
                        } else {
                            self.append_if_content(1)?;
                        }
                    } else if byte == b'\\' {
                        self.append_if_content(1)?;
                        if self.peek(0)?.is_some() {
                            self.append_if_content(1)?;
                        }
                        self.state = State::SingleQuote;
                    } else {
                        self.append_if_content(1)?;
                        self.state = State::SingleQuote;
                    }
                }
                State::DoubleQuote => {
                    if byte == b'"' {
                        if self.peek(1)? == Some(b'"') {
                            self.append_if_content(2)?;
                            self.state = State::DoubleQuote;
                        } else {
                            self.append_if_content(1)?;
                        }
                    } else {
                        self.append_if_content(1)?;
                        self.state = State::DoubleQuote;
                    }
                }
                State::DollarQuote { tag } => {
                    if self.matches(&tag)? {
                        let length = tag.len();
                        self.append(length)?;
                    } else {
                        self.append(1)?;
                        self.state = State::DollarQuote { tag };
                    }
                }
            }
        }
    }

    /// Reads a `COPY ... FROM stdin` data block and writes it to `writer`.
    ///
    /// `on_progress` receives the current byte offset after each line. When it
    /// returns `false`, reading stops and [`CopyDataOutcome::Cancelled`] is
    /// returned.
    pub fn read_copy_data(
        &mut self,
        writer: &mut dyn Write,
        on_progress: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<CopyDataOutcome> {
        skip_copy_header(self)?;

        loop {
            if !on_progress(self.offset) {
                return Ok(CopyDataOutcome::Cancelled);
            }

            if self.is_copy_terminator()? {
                self.consume_copy_terminator()?;
                return Ok(CopyDataOutcome::Completed);
            }

            let wrote_line = self.write_copy_line(writer)?;
            if !wrote_line {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "COPY data ended without a \\. terminator",
                ));
            }
        }
    }

    fn finish(&mut self) -> Result<Option<ScriptStatement>, io::Error> {
        if !self.has_content {
            return Ok(None);
        }
        let statement = self.take_statement()?;
        if statement.sql.is_empty() {
            Ok(None)
        } else {
            Ok(Some(statement))
        }
    }

    fn take_statement(&mut self) -> Result<ScriptStatement, io::Error> {
        let start_line = self.statement_line;
        let end_offset = self.offset;
        let bytes = std::mem::take(&mut self.statement);
        let sql = String::from_utf8(bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
            .trim_end()
            .to_string();
        let copy = is_copy_from_stdin(&sql);

        self.has_content = false;
        self.state = State::Normal;
        self.at_line_start = false;

        Ok(ScriptStatement {
            start_line,
            end_offset,
            sql,
            copy,
        })
    }

    fn append(&mut self, count: usize) -> io::Result<()> {
        self.ensure(count)?;
        let end = self.position + count;
        let bytes = &self.buffer[self.position..end];
        self.statement.extend_from_slice(bytes);
        self.consume(count);
        Ok(())
    }

    fn append_if_content(&mut self, count: usize) -> io::Result<()> {
        if self.has_content {
            self.append(count)
        } else {
            self.ensure(count)?;
            self.consume(count);
            Ok(())
        }
    }

    fn dollar_tag(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut end = 1;
        loop {
            let Some(byte) = self.peek(end)? else {
                return Ok(None);
            };
            if byte == b'$' {
                break;
            }
            if end == 1 && byte.is_ascii_digit() {
                return Ok(None);
            }
            if !(byte.is_ascii_alphanumeric() || byte == b'_') {
                return Ok(None);
            }
            end += 1;
            if end > MAX_DOLLAR_TAG_LEN {
                return Ok(None);
            }
        }

        let mut tag = Vec::with_capacity(end + 1);
        for index in 0..=end {
            let Some(byte) = self.peek(index)? else {
                return Ok(None);
            };
            tag.push(byte);
        }
        Ok(Some(tag))
    }

    fn matches(&mut self, expected: &[u8]) -> io::Result<bool> {
        self.ensure(expected.len())?;
        let end = self.position + expected.len();
        if end > self.filled {
            return Ok(false);
        }
        Ok(&self.buffer[self.position..end] == expected)
    }

    fn is_copy_terminator(&mut self) -> io::Result<bool> {
        if self.peek(0)? != Some(b'\\') || self.peek(1)? != Some(b'.') {
            return Ok(false);
        }
        match self.peek(2)? {
            Some(b'\n') => Ok(true),
            Some(b'\r') => Ok(self.peek(3)? == Some(b'\n')),
            None => Ok(true),
            _ => Ok(false),
        }
    }

    fn consume_copy_terminator(&mut self) -> io::Result<()> {
        match self.peek(2)? {
            Some(b'\n') => self.consume(3),
            Some(b'\r') if self.peek(3)? == Some(b'\n') => self.consume(4),
            _ => self.consume(2),
        }
        Ok(())
    }

    fn write_copy_line(&mut self, writer: &mut dyn Write) -> io::Result<bool> {
        loop {
            self.ensure(1)?;
            let available_len = self.filled - self.position;
            if available_len == 0 {
                return Ok(false);
            }

            let newline = self.buffer[self.position..self.filled]
                .iter()
                .position(|byte| *byte == b'\n');
            if let Some(index) = newline {
                let line = &self.buffer[self.position..self.position + index];
                if let Some(line) = line.strip_suffix(b"\r") {
                    writer.write_all(line)?;
                    writer.write_all(b"\n")?;
                } else {
                    writer.write_all(line)?;
                    writer.write_all(b"\n")?;
                }
                self.consume(index + 1);
                return Ok(true);
            }

            if !self.eof && self.buffer[self.filled - 1] == b'\r' {
                self.ensure(available_len + 1)?;
                if self.filled - self.position > available_len {
                    continue;
                }
            }

            writer.write_all(&self.buffer[self.position..self.position + available_len])?;
            self.consume(available_len);
        }
    }

    fn peek(&mut self, offset: usize) -> io::Result<Option<u8>> {
        self.ensure(offset + 1)?;
        let index = self.position + offset;
        if index >= self.filled {
            return Ok(None);
        }
        Ok(Some(self.buffer[index]))
    }

    fn ensure(&mut self, needed: usize) -> io::Result<()> {
        if self.filled - self.position >= needed {
            return Ok(());
        }
        if needed > self.buffer.len() {
            self.buffer.resize(needed, 0);
        }
        if self.position > 0 {
            let remaining = self.filled - self.position;
            self.buffer.copy_within(self.position..self.filled, 0);
            self.position = 0;
            self.filled = remaining;
        }
        while self.filled < needed && !self.eof {
            let read = self.reader.read(&mut self.buffer[self.filled..])?;
            if read == 0 {
                self.eof = true;
            } else {
                self.filled += read;
            }
        }
        Ok(())
    }

    fn consume(&mut self, count: usize) {
        for _ in 0..count {
            let byte = self.buffer[self.position];
            self.position += 1;
            self.offset += 1;
            if byte == b'\n' {
                self.line += 1;
                self.at_line_start = true;
            } else {
                self.at_line_start = false;
            }
        }
    }
}

fn skip_copy_header<R: Read>(reader: &mut SqlScriptReader<R>) -> io::Result<()> {
    loop {
        match reader.peek(0)? {
            Some(b' ' | b'\t' | b'\r') => reader.consume(1),
            Some(b'\n') => {
                reader.consume(1);
                return Ok(());
            }
            _ => return Ok(()),
        }
    }
}

/// Parses a SQL script into executable chunks. Useful for small inputs and tests.
pub fn parse_sql_script(input: &str) -> Vec<ScriptChunk> {
    let mut reader = SqlScriptReader::new(input.as_bytes());
    let mut chunks = Vec::new();

    while let Some(statement) = reader.next_statement().expect("script is valid UTF-8") {
        let copy_data = if statement.copy {
            let mut data = Vec::new();
            reader
                .read_copy_data(&mut data, &mut |_| true)
                .expect("COPY data can be read");
            Some(String::from_utf8(data).expect("COPY data is valid UTF-8"))
        } else {
            None
        };

        chunks.push(ScriptChunk {
            start_line: statement.start_line,
            end_offset: reader.position(),
            sql: statement.sql,
            copy_data,
        });
    }

    chunks
}

/// True when the statement reads data from standard input.
pub fn is_copy_from_stdin(sql: &str) -> bool {
    let upper = sql.trim().to_ascii_uppercase();
    upper.starts_with("COPY") && upper.contains("FROM STDIN")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn splits_simple_statements() {
        let script = "CREATE TABLE t (id int);\nINSERT INTO t VALUES (1);\n";
        let chunks = parse_sql_script(script);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[0].sql, "CREATE TABLE t (id int);");
        assert_eq!(chunks[1].start_line, 2);
        assert_eq!(chunks[1].sql, "INSERT INTO t VALUES (1);");
    }

    #[test]
    fn keeps_semicolons_inside_quotes_and_comments() {
        let script = "INSERT INTO t VALUES ('a;b'); -- a;b\n/* c;d /* nested; */ */\nSELECT 1;\n";
        let chunks = parse_sql_script(script);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].sql.contains("'a;b'"));
        assert_eq!(chunks[1].sql, "SELECT 1;");
    }

    #[test]
    fn keeps_dollar_quoted_bodies() {
        let script = "CREATE FUNCTION f() RETURNS void AS $body$\nBEGIN\n  PERFORM 1;\nEND;\n$body$ LANGUAGE plpgsql;\nSELECT 2;\n";
        let chunks = parse_sql_script(script);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].sql.contains("PERFORM 1;"));
        assert_eq!(chunks[1].sql, "SELECT 2;");
    }

    #[test]
    fn extracts_copy_data() {
        let script = "COPY public.t (id, name) FROM stdin;\n1\tone\n2\ttwo\n\\.\nSELECT 1;\n";
        let chunks = parse_sql_script(script);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].is_copy());
        assert_eq!(chunks[0].copy_data.as_deref(), Some("1\tone\n2\ttwo\n"));
        assert_eq!(chunks[1].start_line, 5);
        assert_eq!(chunks[1].sql, "SELECT 1;");
    }

    #[test]
    fn normalizes_crlf_copy_data() {
        let script = "COPY t FROM stdin;\r\n1\r\n\\.\r\n";
        let chunks = parse_sql_script(script);
        assert_eq!(chunks[0].copy_data.as_deref(), Some("1\n"));
    }

    #[test]
    fn skips_psql_meta_commands() {
        let script = "\\restrict abc\nSET statement_timeout = 0;\n\\unrestrict abc\n";
        let chunks = parse_sql_script(script);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].sql, "SET statement_timeout = 0;");
        assert_eq!(chunks[0].start_line, 2);
    }

    #[test]
    fn keeps_unterminated_tail() {
        let chunks = parse_sql_script("SELECT 1");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].sql, "SELECT 1");
    }

    #[test]
    fn detects_copy_statement() {
        assert!(is_copy_from_stdin("COPY t FROM stdin;"));
        assert!(is_copy_from_stdin("copy t (a) from stdin"));
        assert!(!is_copy_from_stdin("SELECT 1;"));
    }

    #[test]
    fn parses_across_small_reads() {
        let script = "INSERT INTO t VALUES ('a;b');\nCOPY t FROM stdin;\n1\n2\n\\.\nSELECT 2;\n";
        let reader = SmallReads {
            inner: script.as_bytes(),
            limit: 3,
        };
        let mut reader = SqlScriptReader::new(reader);

        let statement = reader.next_statement().expect("parse").expect("statement");
        assert_eq!(statement.sql, "INSERT INTO t VALUES ('a;b');");

        let statement = reader.next_statement().expect("parse").expect("statement");
        assert!(statement.copy);

        let mut data = Vec::new();
        let outcome = reader
            .read_copy_data(&mut data, &mut |_| true)
            .expect("copy");
        assert_eq!(outcome, CopyDataOutcome::Completed);
        assert_eq!(String::from_utf8(data).expect("utf-8"), "1\n2\n");

        let statement = reader.next_statement().expect("parse").expect("statement");
        assert_eq!(statement.sql, "SELECT 2;");
        assert!(reader.next_statement().expect("parse").is_none());
    }

    #[test]
    fn reads_a_statement_larger_than_the_buffer() {
        let body = "x".repeat(BUFFER_SIZE * 2);
        let script = format!("CREATE TABLE t (data text DEFAULT '{body}');\nSELECT 1;\n");
        let chunks = parse_sql_script(&script);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].sql.starts_with("CREATE TABLE t"));
        assert!(chunks[0].sql.ends_with(");"));
        assert_eq!(chunks[1].sql, "SELECT 1;");
    }

    #[test]
    fn rejects_copy_data_without_terminator() {
        let script = "COPY t FROM stdin;\n1\n2\n";
        let mut reader = SqlScriptReader::new(script.as_bytes());
        let statement = reader.next_statement().expect("parse").expect("statement");
        assert!(statement.copy);

        let mut data = Vec::new();
        let result = reader.read_copy_data(&mut data, &mut |_| true);
        assert!(result.is_err());
    }

    #[test]
    fn stops_copy_data_when_cancelled() {
        let script = "COPY t FROM stdin;\n1\n2\n\\.\nSELECT 1;";
        let mut reader = SqlScriptReader::new(script.as_bytes());
        let statement = reader.next_statement().expect("parse").expect("statement");
        assert!(statement.copy);

        let mut data = Vec::new();
        let mut calls = 0;
        let outcome = reader
            .read_copy_data(&mut data, &mut |_| {
                calls += 1;
                calls < 2
            })
            .expect("copy");
        assert_eq!(outcome, CopyDataOutcome::Cancelled);
        assert_eq!(String::from_utf8(data).expect("utf-8"), "1\n");
    }

    #[test]
    fn reads_only_until_the_first_statement() {
        let content = format!("SELECT 1;\n{}\n", "SELECT 2;\n".repeat(10_000));
        let bytes_read = Arc::new(AtomicUsize::new(0));
        let reader = CountingReads {
            inner: content.as_bytes(),
            bytes_read: Arc::clone(&bytes_read),
        };
        let mut reader = SqlScriptReader::new(reader);
        let statement = reader.next_statement().expect("parse").expect("statement");
        assert_eq!(statement.sql, "SELECT 1;");
        assert!(
            bytes_read.load(Ordering::SeqCst) < content.len(),
            "the whole script must not be read at once"
        );
    }

    struct SmallReads<'a> {
        inner: &'a [u8],
        limit: usize,
    }

    impl Read for SmallReads<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let length = buffer.len().min(self.limit).min(self.inner.len());
            buffer[..length].copy_from_slice(&self.inner[..length]);
            self.inner = &self.inner[length..];
            Ok(length)
        }
    }

    struct CountingReads<'a> {
        inner: &'a [u8],
        bytes_read: Arc<AtomicUsize>,
    }

    impl Read for CountingReads<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let length = buffer.len().min(self.inner.len());
            buffer[..length].copy_from_slice(&self.inner[..length]);
            self.inner = &self.inner[length..];
            self.bytes_read.fetch_add(length, Ordering::SeqCst);
            Ok(length)
        }
    }
}
