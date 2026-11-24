//! Tiny JSON helpers (no dependencies): string quoting and a flat-object reader for request bodies.
use std::collections::HashMap;

pub fn q(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[derive(Debug, Clone)]
pub enum Val {
    S(String),
    N(f64),
    B(bool),
    Null,
}

/// Parses a flat JSON object (string / number / bool / null values only). Returns None on any other shape.
pub fn parse_flat(s: &str) -> Option<HashMap<String, Val>> {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    let ws = |i: &mut usize| {
        while *i < b.len() && b[*i].is_whitespace() {
            *i += 1;
        }
    };
    fn string(b: &[char], i: &mut usize) -> Option<String> {
        if b.get(*i) != Some(&'"') {
            return None;
        }
        *i += 1;
        let mut o = String::new();
        while *i < b.len() {
            let c = b[*i];
            *i += 1;
            match c {
                '"' => return Some(o),
                '\\' => {
                    let e = *b.get(*i)?;
                    *i += 1;
                    match e {
                        '"' => o.push('"'),
                        '\\' => o.push('\\'),
                        '/' => o.push('/'),
                        'n' => o.push('\n'),
                        'r' => o.push('\r'),
                        't' => o.push('\t'),
                        'b' => o.push('\u{8}'),
                        'f' => o.push('\u{c}'),
                        'u' => {
                            let h: String = b.get(*i..*i + 4)?.iter().collect();
                            *i += 4;
                            o.push(char::from_u32(u32::from_str_radix(&h, 16).ok()?)?);
                        }
                        _ => return None,
                    }
                }
                c => o.push(c),
            }
        }
        None
    }
    ws(&mut i);
    if b.get(i) != Some(&'{') {
        return None;
    }
    i += 1;
    let mut m = HashMap::new();
    ws(&mut i);
    if b.get(i) == Some(&'}') {
        return if i + 1 == b.len() || b[i + 1..].iter().all(|c| c.is_whitespace()) { Some(m) } else { None };
    }
    loop {
        ws(&mut i);
        let k = string(&b, &mut i)?;
        ws(&mut i);
        if b.get(i) != Some(&':') {
            return None;
        }
        i += 1;
        ws(&mut i);
        let v = match b.get(i)? {
            '"' => Val::S(string(&b, &mut i)?),
            't' if b[i..].starts_with(&['t', 'r', 'u', 'e']) => { i += 4; Val::B(true) }
            'f' if b[i..].starts_with(&['f', 'a', 'l', 's', 'e']) => { i += 5; Val::B(false) }
            'n' if b[i..].starts_with(&['n', 'u', 'l', 'l']) => { i += 4; Val::Null }
            c if c.is_ascii_digit() || *c == '-' => {
                let st = i;
                while i < b.len() && (b[i].is_ascii_digit() || matches!(b[i], '-' | '+' | '.' | 'e' | 'E')) {
                    i += 1;
                }
                Val::N(b[st..i].iter().collect::<String>().parse().ok()?)
            }
            _ => return None,
        };
        m.insert(k, v);
        ws(&mut i);
        match b.get(i)? {
            ',' => i += 1,
            '}' => {
                i += 1;
                break;
            }
            _ => return None,
        }
    }
    if b[i..].iter().all(|c| c.is_whitespace()) { Some(m) } else { None }
}

pub fn get_str(m: &HashMap<String, Val>, k: &str) -> Option<String> {
    match m.get(k) {
        Some(Val::S(s)) => Some(s.clone()),
        _ => None,
    }
}
pub fn get_u64(m: &HashMap<String, Val>, k: &str) -> Option<u64> {
    match m.get(k) {
        Some(Val::N(n)) if *n >= 0.0 && n.fract() == 0.0 && *n < 9.0e15 => Some(*n as u64),
        _ => None,
    }
}
