pub fn is_z(c: char) -> bool {
    matches!(c, '\0')
}

pub fn is_break(c: char) -> bool {
    matches!(c, '\n' | '\r')
}

pub fn is_blank(c: char) -> bool {
    matches!(c, ' ' | '\t')
}

pub fn is_digit(c: char) -> bool {
    ('0'..='9').contains(&c)
}

pub fn is_alpha(c: char) -> bool {
    matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' | '-')
}

pub fn is_hex(c: char) -> bool {
    ('0'..='9').contains(&c) || ('a'..='f').contains(&c) || ('A'..='F').contains(&c)
}

pub fn is_breakz(c: char) -> bool {
    is_break(c) || is_z(c)
}

pub fn is_blankz(c: char) -> bool {
    is_blank(c) || is_breakz(c)
}

pub fn is_flow(c: char) -> bool {
    matches!(c, ',' | '[' | ']' | '{' | '}')
}

pub fn as_hex(c: char) -> u32 {
    match c {
        '0'..='9' => (c as u32) - ('0' as u32),
        'a'..='f' => (c as u32) - ('a' as u32) + 10,
        'A'..='F' => (c as u32) - ('A' as u32) + 10,
        _ => unreachable!(),
    }
}
