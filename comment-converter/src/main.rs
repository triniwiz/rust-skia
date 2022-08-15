use anyhow::Result;
use heck::{ToLowerCamelCase, ToSnakeCase, ToUpperCamelCase};
use std::{
    io::{self, Read, Write},
    str,
};

fn main() -> Result<()> {
    let mut stdin = io::stdin();
    let mut buf = String::new();
    stdin.read_to_string(&mut buf)?;
    let result = convert(&buf);
    io::stdout().write_all(result.as_bytes())?;
    Ok(())
}

fn convert(source: &str) -> String {
    let line_trimmed = remove_lines(source, remove_line);
    let line_trimmed = {
        let mut lines: Vec<_> = lines(&line_trimmed)
            .into_iter()
            .map(|l| l.to_string())
            .collect();
        if !lines.is_empty() {
            lines[0] = first_line_converter(lines[0].as_str());
        }
        lines.join("\n")
    };
    let trimmed = trim_common_indent(&line_trimmed, |b| b == b' ');
    let trimmed = trim_common_indent(&trimmed, |b| b == b'*');
    let trimmed = trim_common_indent(&trimmed, |b| b == b' ');
    let tokens = tokenize(&trimmed);
    let processed = process_tokens(&tokens);
    comment(&processed)
}

fn comment(str: &str) -> String {
    let mut lines: Vec<_> = lines(str).into_iter().map(|l| l.to_string()).collect();

    for i in 0..lines.len() {
        let is_last = i == lines.len() - 1;

        let line = &mut lines[i];
        let convert = !((is_last && line.trim().is_empty()) || line.starts_with("/// "));
        if convert {
            *line = format!("/// {line}");
        }
    }

    lines.join("\n")
}

fn remove_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "/**" || trimmed == "*/" ||
    // like `/** \class SkPath`
    trimmed.starts_with("/** \\")
}

/// Trims the common indent of all lines.
fn trim_common_indent(source: &str, is_indent: impl Fn(u8) -> bool) -> String {
    let min_indent = source
        .lines()
        .filter_map(|line| indent_size(line, &is_indent))
        .min()
        .unwrap_or_default();

    let lines: Vec<String> = lines(source)
        .into_iter()
        .map(|str| {
            String::from_utf8(str.bytes().skip(min_indent).collect::<Vec<u8>>())
                .expect("Internal Error, UTF8 conversion failed")
        })
        .collect();
    lines.join("\n")
}

/// Symmetric line splitting.
fn lines(all: &str) -> Vec<&str> {
    let mut r: Vec<_> = all.lines().collect();
    if all.ends_with('\n') {
        r.push("");
    }
    r
}

/// This replaces a `/** ` by spaces when it appears in the first line after an optional indent.
fn first_line_converter(first_line: &str) -> String {
    let indent_size = indent_size(first_line, |b| b == b' ').unwrap_or_default();
    if first_line[indent_size..].starts_with("/** ") {
        let indent = &first_line[0..indent_size];
        let rest = &first_line[indent_size + 4..];
        return format!("{indent}    {rest}");
    }

    first_line.to_string()
}

fn remove_lines(source: &str, remove_if: impl Fn(&str) -> bool) -> String {
    let lines: Vec<String> = lines(source)
        .into_iter()
        .filter_map(|line| {
            if remove_if(line) {
                None
            } else {
                Some(line.to_string())
            }
        })
        .collect();
    lines.join("\n")
}

fn process_tokens(tokens: &[Token]) -> String {
    let mut r = String::new();
    let mut current = 0;
    let tokens: Vec<_> = tokens.iter().map(|t| t.as_ref()).collect();

    while current != tokens.len() {
        let (consumed, str) = consume_tokens(&tokens[current..]);
        current += consumed;
        assert!(current <= tokens.len());
        r += &str;
    }
    r
}

fn consume_tokens(tokens: &[RefToken]) -> (usize, String) {
    use RefToken::*;
    match tokens {
        [Word("@param"), Whitespace(" "), Word(name), ..] => (3, format!("- `{name}` ")),
        [Word("@return"), Whitespace(_), Word(_), ..] => (2, "Returns: ".into()),
        [Word(word), ..] => {
            if let Some(reference) = word.strip_prefix("Sk") {
                let reference = convert_sk_reference(reference);
                return (1, format!("[`{reference}`]"));
            }
            if word.starts_with("https://") {
                return (1, format!("<{word}>"));
            }
            if *word == "true" || *word == "false" {
                return (1, format!("`{word}`"));
            }
            if let Some(new_function_name) = convert_c_function(word) {
                return (1, format!("`{new_function_name}`"));
            }
            (1, word.to_string())
        }
        [Whitespace(ws), ..] => (1, ws.to_string()),
        [Separator(sep), ..] => (1, sep.to_string()),
        [] => panic!("Internal error"),
    }
}

fn convert_c_function(word: &str) -> Option<String> {
    if let Some(fn_name) = word.strip_suffix("()") {
        if fn_name.to_lower_camel_case() == fn_name {
            return Some(fn_name.to_snake_case() + "()");
        }
    }
    None
}

/// Converts references like `Path::updateBoundsCache` or `Path::Verb`.
fn convert_sk_reference(reference: &str) -> String {
    if let Some((type_name, sub_name)) = reference.split_once("::") {
        // Nested type: like Path::Verb -> path::Verb.
        if sub_name.to_upper_camel_case() == sub_name {
            let module_name = type_name.to_snake_case();
            return format!("{module_name}::{sub_name}");
        }
        // Nested function: like `Path::updateBoundsCache`
        if sub_name.to_lower_camel_case() == sub_name {
            let fun_name = sub_name.to_snake_case();
            return format!("{type_name}::{fun_name}");
        }
    }
    reference.into()
}

fn indent_size(source: &str, is_indent: impl Fn(u8) -> bool) -> Option<usize> {
    source.bytes().position(|b| !is_indent(b))
}

/// A token in the original comment.
enum Token {
    Word(String),
    Whitespace(String),
    /// Phrase separator only, ,.;
    Separator(String),
}

impl Token {
    pub fn as_ref(&self) -> RefToken {
        match self {
            Token::Word(w) => RefToken::Word(w),
            Token::Whitespace(ws) => RefToken::Whitespace(ws),
            Token::Separator(ws) => RefToken::Separator(ws),
        }
    }
}

enum RefToken<'a> {
    Word(&'a str),
    Whitespace(&'a str),
    Separator(&'a str),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum TokenClass {
    Word,
    Whitespace,
    Separator,
}

impl TokenClass {
    pub fn classify(c: char) -> TokenClass {
        if c == '.' || c == ';' || c == ',' {
            return TokenClass::Separator;
        }
        if c.is_whitespace() {
            return TokenClass::Whitespace;
        }
        TokenClass::Word
    }

    pub fn to_token(self, str: &str) -> Token {
        assert!(!str.is_empty());
        match self {
            TokenClass::Word => Token::Word(str.into()),
            TokenClass::Whitespace => Token::Whitespace(str.into()),
            TokenClass::Separator => Token::Separator(str.into()),
        }
    }
}

fn tokenize(source: &str) -> Vec<Token> {
    let mut current = None;
    let mut str = String::new();
    let mut r = Vec::new();

    for (_, c) in source.chars().enumerate() {
        let token_class = TokenClass::classify(c);
        if current != Some(token_class) {
            if let Some(current) = current {
                r.push(current.to_token(&str));
            }
            current = Some(token_class);
            str = c.to_string()
        } else {
            str.push(c)
        }
    }

    if let Some(last) = current {
        r.push(last.to_token(&str));
    }

    r
}

#[cfg(test)]
mod cfg {
    use super::lines;
    use rstest::rstest;

    #[rstest]
    #[case("")]
    #[case("a")]
    #[case("a\n")]
    #[case("a\nb")]
    #[case("a\nb\n\nc")]
    #[case("a\nb\n\nc\n")]
    #[case("a\nb\n\nc\n\n")]
    pub fn lines_splitting_is_symmetric(#[case] v: &str) {
        let r = lines(v).join("\n");
        assert_eq!(v, r)
    }
}
