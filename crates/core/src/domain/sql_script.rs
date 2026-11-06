//! Splits a PostgreSQL plain-text backup into executable chunks.
//!
//! The splitter understands quoted strings, dollar-quoted bodies, comments
//! and `COPY ... FROM stdin` data blocks. It also ignores psql meta-commands
//! that cannot be executed by a database client.

/// One executable piece of a SQL script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptChunk {
    /// Line in the source file where the chunk starts.
    pub start_line: usize,
    /// Byte offset just after the chunk in the source file.
    pub end_offset: usize,
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

/// Parses a SQL script into executable chunks.
pub fn parse_sql_script(input: &str) -> Vec<ScriptChunk> {
    let bytes = input.as_bytes();
    let mut chunks = Vec::new();
    let mut index = 0;
    let mut statement_start = 0;
    let mut statement_line = 1;
    let mut line = 1;
    let mut at_line_start = true;
    let mut has_content = false;

    while index < bytes.len() {
        let byte = bytes[index];

        // psql meta-commands are not valid SQL. Skip a line that starts with
        // a backslash when no statement content is pending.
        if at_line_start && byte == b'\\' && !has_content {
            let end = find_line_end(input, index);
            statement_start = end;
            statement_line = line;
            index = end;
            continue;
        }

        if !byte.is_ascii_whitespace()
            && byte != b';'
            && !has_content
            && !is_comment_start(bytes, index)
        {
            statement_start = index;
            statement_line = line;
            has_content = true;
        }

        match byte {
            b'\n' => {
                line += 1;
                index += 1;
                at_line_start = true;
            }
            b'-' if bytes.get(index + 1) == Some(&b'-') => {
                index = find_line_end(input, index);
                at_line_start = false;
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                let (end, newlines) = skip_block_comment(input, index);
                line += newlines;
                index = end;
                at_line_start = false;
            }
            b'\'' => {
                let (end, newlines) = skip_single_quoted(input, index);
                line += newlines;
                index = end;
                at_line_start = false;
            }
            b'"' => {
                let (end, newlines) = skip_double_quoted(input, index);
                line += newlines;
                index = end;
                at_line_start = false;
            }
            b'$' => {
                if let Some((end, newlines)) = skip_dollar_quoted(input, index) {
                    line += newlines;
                    index = end;
                } else {
                    index += 1;
                }
                at_line_start = false;
            }
            b';' => {
                let sql = input[statement_start..=index].trim();
                if has_content && !sql.is_empty() {
                    let sql = sql.to_string();
                    if is_copy_from_stdin(&sql) {
                        let (data, end, newlines) = read_copy_data(input, index + 1);
                        chunks.push(ScriptChunk {
                            start_line: statement_line,
                            end_offset: end,
                            sql,
                            copy_data: Some(data),
                        });
                        line += newlines;
                        index = end;
                    } else {
                        chunks.push(ScriptChunk {
                            start_line: statement_line,
                            end_offset: index + 1,
                            sql,
                            copy_data: None,
                        });
                        index += 1;
                    }
                } else {
                    index += 1;
                }
                has_content = false;
                statement_start = index;
                statement_line = line;
                at_line_start = false;
            }
            _ => {
                index += 1;
                at_line_start = false;
            }
        }
    }

    let tail = input[statement_start..].trim();
    if has_content && !tail.is_empty() {
        chunks.push(ScriptChunk {
            start_line: statement_line,
            end_offset: input.len(),
            sql: tail.to_string(),
            copy_data: None,
        });
    }

    chunks
}

/// True when the statement reads data from standard input.
pub fn is_copy_from_stdin(sql: &str) -> bool {
    let upper = sql.trim().to_ascii_uppercase();
    upper.starts_with("COPY") && upper.contains("FROM STDIN")
}

fn find_line_end(input: &str, start: usize) -> usize {
    input[start..]
        .find('\n')
        .map_or(input.len(), |offset| start + offset)
}

fn is_comment_start(bytes: &[u8], index: usize) -> bool {
    let byte = bytes[index];
    matches!(
        (byte, bytes.get(index + 1)),
        (b'-', Some(b'-')) | (b'/', Some(b'*'))
    )
}

fn newlines_in(text: &str) -> usize {
    text.bytes().filter(|byte| *byte == b'\n').count()
}

fn skip_single_quoted(input: &str, start: usize) -> (usize, usize) {
    let bytes = input.as_bytes();
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' => {
                if bytes.get(index + 1) == Some(&b'\'') {
                    index += 2;
                } else {
                    index += 1;
                    break;
                }
            }
            b'\\' => index += 2,
            _ => index += 1,
        }
    }
    let end = index.min(bytes.len());
    (end, newlines_in(&input[start..end]))
}

fn skip_double_quoted(input: &str, start: usize) -> (usize, usize) {
    let bytes = input.as_bytes();
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            if bytes.get(index + 1) == Some(&b'"') {
                index += 2;
            } else {
                index += 1;
                break;
            }
        } else {
            index += 1;
        }
    }
    let end = index.min(bytes.len());
    (end, newlines_in(&input[start..end]))
}

fn skip_block_comment(input: &str, start: usize) -> (usize, usize) {
    let bytes = input.as_bytes();
    let mut index = start + 2;
    let mut depth = 1usize;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            depth += 1;
            index += 2;
        } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
            depth -= 1;
            index += 2;
            if depth == 0 {
                break;
            }
        } else {
            index += 1;
        }
    }
    let end = index.min(bytes.len());
    (end, newlines_in(&input[start..end]))
}

fn skip_dollar_quoted(input: &str, start: usize) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let mut tag_end = start + 1;
    while tag_end < bytes.len()
        && (bytes[tag_end].is_ascii_alphanumeric() || bytes[tag_end] == b'_')
    {
        tag_end += 1;
    }
    if bytes.get(tag_end) != Some(&b'$') {
        return None;
    }
    if tag_end > start + 1 && bytes[start + 1].is_ascii_digit() {
        return None;
    }

    let tag = &input[start..=tag_end];
    let body_start = tag_end + 1;
    let end = match input[body_start..].find(tag) {
        Some(offset) => body_start + offset + tag.len(),
        None => input.len(),
    };
    Some((end, newlines_in(&input[start..end])))
}

fn read_copy_data(input: &str, from: usize) -> (String, usize, usize) {
    let bytes = input.as_bytes();
    let mut data_start = from;
    while data_start < bytes.len() && matches!(bytes[data_start], b' ' | b'\t' | b'\r') {
        data_start += 1;
    }
    if bytes.get(data_start) == Some(&b'\n') {
        data_start += 1;
    }

    let mut line_start = data_start;
    while line_start < bytes.len() {
        let line_end = find_line_end(input, line_start);
        let content = input[line_start..line_end].trim_end_matches('\r');
        if content == "\\." {
            let data = normalize_copy_data(&input[data_start..line_start]);
            let end = if line_end < bytes.len() {
                line_end + 1
            } else {
                line_end
            };
            return (data, end, newlines_in(&input[from..end]));
        }
        if line_end >= bytes.len() {
            break;
        }
        line_start = line_end + 1;
    }

    let data = normalize_copy_data(&input[data_start..]);
    (data, input.len(), newlines_in(&input[from..]))
}

fn normalize_copy_data(data: &str) -> String {
    data.replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
