pub use self::error::EmitError;
use self::funcs::escape_str;
use self::funcs::need_quotes;
use crate::yaml::Hash;
use crate::yaml::IntegerFormat;
use crate::yaml::Meta;
use crate::yaml::StringFormat;
use crate::yaml::Yaml;
use std::fmt;

mod error;
mod funcs;

pub struct YamlEmitter<'a> {
    writer: &'a mut dyn fmt::Write,
    best_indent: usize,
    compact: bool,
    level: isize,
    intformat: IntegerFormat,
    strformat: StringFormat,
}

pub type EmitResult = Result<(), EmitError>;

impl<'a> YamlEmitter<'a> {
    pub fn new(writer: &'a mut dyn fmt::Write) -> YamlEmitter {
        YamlEmitter {
            writer,
            best_indent: 2,
            compact: true,
            level: -1,
            intformat: IntegerFormat::Decimal,
            strformat: StringFormat::Standard,
        }
    }

    /// Set 'compact inline notation' on or off, as described for block
    /// [sequences](http://www.yaml.org/spec/1.2/spec.html#id2797382)
    /// and
    /// [mappings](http://www.yaml.org/spec/1.2/spec.html#id2798057).
    ///
    /// In this form, blocks cannot have any properties (such as anchors
    /// or tags), which should be OK, because this emitter doesn't
    /// (currently) emit those anyways.
    pub fn compact(&mut self, compact: bool) {
        self.compact = compact;
    }

    /// Determine if this emitter is using 'compact inline notation'.
    pub fn is_compact(&self) -> bool {
        self.compact
    }

    pub fn dump(&mut self, doc: &'a Yaml) -> EmitResult {
        write!(self.writer, "---")?;

        // Emits comments inlined after document beginning
        if let Yaml::Array(arr) = doc {
            if let Some(first) = arr.first() {
                if first.is_inline_comment() {
                    self.emit_node(first)?;
                }
            }
        } else if let Yaml::Hash(hash) = doc {
            if let Some((first, _)) = hash.front() {
                if first.is_inline_comment() {
                    self.emit_node(first)?;
                }
            }
        }

        writeln!(self.writer)?;

        self.level = -1;
        self.emit_node(doc)
    }

    fn emit_node(&mut self, node: &'a Yaml) -> EmitResult {
        match *node {
            Yaml::Array(ref v) => self.emit_array(v),
            Yaml::Hash(ref v) => self.emit_hash(v),
            Yaml::String(ref v) => self.emit_string(v.as_str()),
            Yaml::Boolean(v) => {
                match v {
                    true => write!(self.writer, "true")?,
                    false => write!(self.writer, "false")?,
                }
                Ok(())
            }
            Yaml::Integer(v) => self.emit_integer(v),
            Yaml::Real(ref v) => {
                write!(self.writer, "{}", v)?;
                Ok(())
            }
            Yaml::Comment(ref comment, inline) => {
                match inline {
                    true => write!(self.writer, " #{}", comment)?,
                    false => {
                        writeln!(self.writer, "#{}", comment)?;
                        self.emit_indent()?;
                    }
                }
                Ok(())
            }
            Yaml::Null | Yaml::BadValue => {
                write!(self.writer, "~")?;
                Ok(())
            }
            Yaml::Alias(_) => Ok(()),
            Yaml::Meta(ref op) => self.process_meta(op),
        }
    }

    fn process_meta(&mut self, op: &'a Meta) -> EmitResult {
        match op {
            Meta::Fragment(frag) => {
                for f in frag {
                    self.emit_node(f)?;
                }
                Ok(())
            }
            Meta::Integer(f, node) => {
                let old = self.intformat;
                self.intformat = *f;
                let res = self.emit_node(node);
                self.intformat = old;
                res
            }
            Meta::String(f, node) => {
                let old = self.strformat;
                self.strformat = *f;
                let res = self.emit_node(node);
                self.strformat = old;
                res
            }
            Meta::Int128(val) => self.emit_i128(*val),
        }
    }

    fn emit_string(&mut self, mut value: &str) -> EmitResult {
        match self.strformat {
            StringFormat::Standard => {
                if need_quotes(value) {
                    escape_str(self.writer, value, true)?;
                } else {
                    write!(self.writer, "{}", value)?;
                }
            }
            StringFormat::Quoted => escape_str(self.writer, value, true)?,
            StringFormat::Block => {
                if value.ends_with('\n') {
                    writeln!(self.writer, "|+")?;
                    value = &value[..value.len() - 1];
                } else {
                    writeln!(self.writer, "|-")?;
                }
                self.level += 1;
                for (count, line) in value.split('\n').enumerate() {
                    if count > 0 {
                        writeln!(self.writer)?;
                    }
                    if !line.is_empty() {
                        self.emit_indent()?;
                        escape_str(self.writer, line, false)?;
                    }
                }
                self.level -= 1;
            }
        }
        Ok(())
    }

    fn emit_integer(&mut self, value: i64) -> EmitResult {
        const BUFSZ: usize = 64 + 2;
        let (bits, width, base) = match self.intformat {
            IntegerFormat::Binary(bits, w) => (bits, w as usize, 2),
            IntegerFormat::Decimal => {
                write!(self.writer, "{}", value)?;
                return Ok(());
            }
            IntegerFormat::Hex(bits, w) => (bits, w as usize, 16),
            IntegerFormat::Octal(bits, w) => (bits, w as usize, 8),
        };
        if width > BUFSZ - 2 {
            return Err(EmitError::IntFmtWidth);
        }
        let mut s = [b' '; BUFSZ];
        let mut i = BUFSZ;
        let mut value = value as u64;
        if bits < 64 {
            value &= (1u64 << bits) - 1;
        }
        loop {
            i -= 1;
            match value % base {
                x if x < 10 => s[i] = b'0' + (x as u8),
                x => s[i] = b'A' + x as u8 - 10,
            };
            value /= base;
            if value == 0 {
                break;
            }
        }
        while BUFSZ - i < width {
            i -= 1;
            s[i] = b'0';
        }
        match self.intformat {
            IntegerFormat::Binary(_, _) => {
                i -= 2;
                s[i] = b'0';
                s[i + 1] = b'b';
            }
            IntegerFormat::Hex(_, _) => {
                i -= 2;
                s[i] = b'0';
                s[i + 1] = b'x';
            }
            IntegerFormat::Octal(_, _) => {
                i -= 2;
                s[i] = b'0';
                s[i + 1] = b'o';
            }
            IntegerFormat::Decimal => {}
        };
        write!(self.writer, "{}", std::str::from_utf8(&s[i..]).unwrap())?;
        Ok(())
    }

    fn emit_i128(&mut self, value: i128) -> EmitResult {
        const BUFSZ: usize = 128 + 2;
        let (bits, width, base) = match self.intformat {
            IntegerFormat::Binary(bits, w) => (bits, w as usize, 2),
            IntegerFormat::Decimal => {
                write!(self.writer, "{}", value)?;
                return Ok(());
            }
            IntegerFormat::Hex(bits, w) => (bits, w as usize, 16),
            IntegerFormat::Octal(bits, w) => (bits, w as usize, 8),
        };
        if width > BUFSZ - 2 {
            return Err(EmitError::IntFmtWidth);
        }
        let mut s = [b' '; BUFSZ];
        let mut i = BUFSZ;
        let mut value = value as u128;
        if bits < 128 {
            value &= (1u128 << bits) - 1;
        }
        loop {
            i -= 1;
            match value % base {
                x if x < 10 => s[i] = b'0' + (x as u8),
                x => s[i] = b'A' + x as u8 - 10,
            };
            value /= base;
            if value == 0 {
                break;
            }
        }
        while BUFSZ - i < width {
            i -= 1;
            s[i] = b'0';
        }
        match self.intformat {
            IntegerFormat::Binary(_, _) => {
                i -= 2;
                s[i] = b'0';
                s[i + 1] = b'b';
            }
            IntegerFormat::Hex(_, _) => {
                i -= 2;
                s[i] = b'0';
                s[i + 1] = b'x';
            }
            IntegerFormat::Octal(_, _) => {
                if s[i] != b'0' {
                    i -= 1;
                    s[i] = b'0';
                }
            }
            IntegerFormat::Decimal => {}
        };
        write!(self.writer, "{}", std::str::from_utf8(&s[i..]).unwrap())?;
        Ok(())
    }

    fn emit_array(&mut self, arr: &'a [Yaml]) -> EmitResult {
        if arr.is_empty() {
            write!(self.writer, "[]")?;
            return Ok(());
        }

        self.level += 1;
        let mut idx = -1;
        let mut iter = arr.iter().peekable();
        while let Some(entry) = iter.next() {
            // The only way the first entry is an inlined comment is because
            // the comment belongs to the parent. Ignore it.
            if idx == -1 && entry.is_inline_comment() {
                continue;
            }

            idx += 1;
            if idx > 0 {
                self.emit_line_begin()?;
            }

            if entry.is_comment() {
                self.emit_node(entry)?;
                continue;
            }
            let entry = if let Some(f) = entry.get_comment_fragment() {
                self.emit_node(&f[0])?;
                &f[1]
            } else {
                entry
            };
            write!(self.writer, "-")?;
            self.emit_value(true, entry)?;

            if let Some(entry) = iter.next_if(|entry| entry.is_inline_comment()) {
                self.emit_node(entry)?;
            }
        }
        self.level -= 1;
        Ok(())
    }

    fn emit_hash(&mut self, hash: &'a Hash) -> EmitResult {
        if hash.is_empty() {
            self.writer.write_str("{}")?;
            return Ok(());
        }

        self.level += 1;
        let mut idx = -1;
        let mut iter = hash.iter().peekable();
        while let Some((key, value)) = iter.next() {
            // The only way the first entry is an inlined comment is because
            // the comment belongs to the parent. Ignore it.
            if idx == -1 && key.is_inline_comment() {
                continue;
            }

            idx += 1;
            if idx > 0 {
                self.emit_line_begin()?;
            }

            if key.is_comment() {
                self.emit_node(key)?;
                continue;
            }

            let is_complex_key = matches!(*key, Yaml::Hash(_) | Yaml::Array(_));
            if is_complex_key {
                write!(self.writer, "?")?;
                self.emit_value(true, key)?;
                self.emit_line_begin()?;
                write!(self.writer, ":")?;
                self.emit_value(true, value)?;
            } else {
                self.emit_node(key)?;
                write!(self.writer, ":")?;
                self.emit_value(false, value)?;
            }

            if let Some((key, _)) = iter.next_if(|(key, _)| key.is_inline_comment()) {
                self.emit_node(key)?;
            }
        }
        self.level -= 1;
        Ok(())
    }

    /// Emit a yaml as a hash or array value: i.e., which should appear
    /// following a ":" or "-", either after a space, or on a new line.
    /// If `inline` is true, then the preceding characters are distinct
    /// and short enough to respect the compact flag.
    fn emit_value(&mut self, inline: bool, value: &'a Yaml) -> EmitResult {
        match *value {
            Yaml::Array(ref arr) => {
                if arr.is_empty() {
                    write!(self.writer, " []")?;
                    return Ok(());
                }

                // Emit inlined comment before starting to spit out the array
                // If the first entry is an inlined comment, it belongs to
                // the parent hash key / array entry.
                let mut from = 0;
                if arr[0].is_inline_comment() {
                    self.emit_node(&arr[0])?;
                    from = 1;
                }

                self.emit_value_indent(inline)?;
                self.emit_array(&arr[from..])
            }
            Yaml::Hash(ref hash) => {
                if hash.is_empty() {
                    self.writer.write_str(" {}")?;
                    return Ok(());
                }

                // Emit inlined comment before starting to spit out the hash
                // If the first entry is an inlined comment, it belongs to
                // the parent hash key / array entry.
                if let Some((key, _)) = hash.front() {
                    if key.is_inline_comment() {
                        self.emit_node(key)?;
                    }
                }

                self.emit_value_indent(inline)?;
                self.emit_hash(hash)
            }
            Yaml::Comment(_, _) => {
                unreachable!("should never emit comment as a value: {:?}", value)
            }
            _ => {
                write!(self.writer, " ")?;
                self.emit_node(value)
            }
        }
    }

    fn emit_line_begin(&mut self) -> EmitResult {
        writeln!(self.writer)?;
        self.emit_indent()?;
        Ok(())
    }

    fn emit_value_indent(&mut self, inline: bool) -> EmitResult {
        if inline && self.compact {
            write!(self.writer, " ")?;
        } else {
            writeln!(self.writer)?;
            self.level += 1;
            self.emit_indent()?;
            self.level -= 1;
        }
        Ok(())
    }

    fn emit_indent(&mut self) -> EmitResult {
        for _ in 0..(self.level * self.best_indent as isize) {
            write!(self.writer, " ")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::YamlLoader;
    use pretty_assertions::assert_eq;
    use std::env;
    use std::fs;

    macro_rules! fixture_test {
        ($test_name:ident, $fixture:expr) => {
            #[test]
            fn $test_name() -> Result<(), std::io::Error> {
                let record_fixtures = env::var("RECORD_FIXTURES").is_ok();

                let input = format!("tests/fixtures/{}.input.yaml", $fixture);
                let expected = format!("tests/fixtures/{}.expected.yaml", $fixture);
                fixture_roundtrip(&input, &expected, record_fixtures);
                Ok(())
            }
        };
    }

    #[test]
    fn test_empty_and_nested() {
        let input = r#"---
a:
  b:
    c: hello
  d: {}
e:
  - f
  - g
  - h: []"#;

        let noncompact_input = r#"---
a:
  b:
    c: hello
  d: {}
e:
  - f
  - g
  -
    h: []"#;

        assert_roundtrip(input);
        assert_roundtrip_noncompact(noncompact_input);
    }

    #[test]
    fn test_nested_arrays() {
        let input = r#"---
a:
  - b
  - - c
    - d
    - - e
      - f"#;

        assert_roundtrip(input);
    }

    #[test]
    fn test_deeply_nested_arrays() {
        let input = r#"---
a:
  - b
  - - c
    - d
    - - e
      - - f
      - - e"#;

        assert_roundtrip(input);
    }

    #[test]
    fn test_nested_hashes() {
        let input = r#"---
a:
  b:
    c:
      d:
        e: f"#;

        assert_roundtrip(input);
    }

    fn yaml_string(s: &str) -> Yaml {
        Yaml::String(s.to_string())
    }
    fn yaml_fmtstr(s: &str, f: StringFormat) -> Yaml {
        Yaml::Meta(Meta::String(f, yaml_string(s).into()))
    }
    fn yaml_integer(v: i64, f: IntegerFormat) -> Yaml {
        Yaml::Meta(Meta::Integer(f, Yaml::Integer(v).into()))
    }
    fn yaml_i128(v: i128, f: IntegerFormat) -> Yaml {
        Yaml::Meta(Meta::Integer(f, Yaml::Meta(Meta::Int128(v)).into()))
    }
    fn yaml_dump(doc: &Yaml) -> String {
        let mut writer = String::new();
        let mut emitter = YamlEmitter::new(&mut writer);
        emitter.dump(&doc).unwrap();
        writer
    }

    #[test]
    fn test_integer_bases_simple() {
        let expected = r#"---
dec: 10
bin: 0b10
hex: 0x10
oct: 0o10"#;
        let mut hash = Hash::new();
        hash.insert(yaml_string("dec"), yaml_integer(10, IntegerFormat::Decimal));
        hash.insert(
            yaml_string("bin"),
            yaml_integer(2, IntegerFormat::Binary(32, 0)),
        );
        hash.insert(
            yaml_string("hex"),
            yaml_integer(16, IntegerFormat::Hex(32, 0)),
        );
        hash.insert(
            yaml_string("oct"),
            yaml_integer(8, IntegerFormat::Octal(32, 0)),
        );
        let result = yaml_dump(&Yaml::Hash(hash));
        assert_eq!(expected, result);
    }

    #[test]
    fn test_integer_bases_widths() {
        let expected = r#"---
bin: 0b00001111
hex: 0x0000ABCD
oct: 0o666"#;
        let mut hash = Hash::new();
        hash.insert(
            yaml_string("bin"),
            // 32-bit integer padded to 8 palces.
            yaml_integer(15, IntegerFormat::Binary(32, 8)),
        );
        hash.insert(
            yaml_string("hex"),
            // 32-bit integer padded to 8 palces.
            yaml_integer(0xABCD, IntegerFormat::Hex(32, 8)),
        );
        hash.insert(
            yaml_string("oct"),
            // 32-bit integer padded to 3 palces.
            yaml_integer(0o666, IntegerFormat::Octal(32, 3)),
        );
        let result = yaml_dump(&Yaml::Hash(hash));
        assert_eq!(expected, result);
    }

    #[test]
    fn test_integer_bases_negative() {
        let expected = r#"---
bin: 0b11110000
hex: 0xFFFFFFFE
oct: 0o177771"#;
        let mut hash = Hash::new();
        hash.insert(
            yaml_string("bin"),
            // 8-bit integer, padded to 8 places.
            yaml_integer(-16, IntegerFormat::Binary(8, 8)),
        );
        hash.insert(
            yaml_string("hex"),
            // 32-bit integer, padded to 8 places.
            yaml_integer(-2, IntegerFormat::Hex(32, 8)),
        );
        hash.insert(
            yaml_string("oct"),
            // 16-bit integer, padded to 8 places; should use more than 3
            // spaces because of leading 1-bits.
            yaml_integer(-7, IntegerFormat::Octal(16, 3)),
        );
        let result = yaml_dump(&Yaml::Hash(hash));
        assert_eq!(expected, result);
    }

    #[test]
    fn test_integer_128() {
        let expected = r#"---
dec: 10
bin: 0b00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000001
hex: 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF80
oct: 0234200"#;
        let mut hash = Hash::new();
        hash.insert(yaml_string("dec"), yaml_i128(10, IntegerFormat::Decimal));
        hash.insert(
            yaml_string("bin"),
            yaml_i128(65537, IntegerFormat::Binary(128, 128)),
        );
        hash.insert(
            yaml_string("hex"),
            yaml_i128(-128, IntegerFormat::Hex(128, 0)),
        );
        hash.insert(
            yaml_string("oct"),
            yaml_i128(80000, IntegerFormat::Octal(128, 0)),
        );
        let result = yaml_dump(&Yaml::Hash(hash));
        assert_eq!(expected, result);
    }

    #[test]
    fn test_block_scalar() {
        let expected = r#"---
a: |+
  This is a block scalar.
  There are two newlines at the end of this string.

b: |-
  This is a block scalar.
  There is no newline at the end of this string.
c: "A plain string, forcibly quoted"
d: no quotes needed
e: "yes,\nquotes needed""#;

        let mut hash = Hash::new();
        hash.insert(
            yaml_string("a"),
            yaml_fmtstr(
                "This is a block scalar.\nThere are two newlines at the end of this string.\n\n",
                StringFormat::Block,
            ),
        );

        hash.insert(
            yaml_string("b"),
            yaml_fmtstr(
                "This is a block scalar.\nThere is no newline at the end of this string.",
                StringFormat::Block,
            ),
        );

        hash.insert(
            yaml_string("c"),
            yaml_fmtstr("A plain string, forcibly quoted", StringFormat::Quoted),
        );

        hash.insert(yaml_string("d"), yaml_string("no quotes needed"));
        hash.insert(yaml_string("e"), yaml_string("yes,\nquotes needed"));

        let result = yaml_dump(&Yaml::Hash(hash));
        assert_eq!(expected, result);
    }

    fixture_test!(test_emit_simple, "emitter/simple");

    fixture_test!(test_emit_complex, "emitter/complex");

    fixture_test!(test_emit_avoid_quotes, "emitter/avoid-quotes");

    fixture_test!(test_emit_quoted_bools, "emitter/quoted-bools");

    fixture_test!(test_comments_001, "emitter/comments-001");

    fixture_test!(test_comments_002, "emitter/comments-002");

    fixture_test!(test_comments_hash, "emitter/comments-hash");
    fixture_test!(test_comments_hash_deep, "emitter/comments-hash-deep");

    fixture_test!(test_comments_array, "emitter/comments-array");
    fixture_test!(test_comments_array_deep, "emitter/comments-array-deep");

    // Asserts the roundtrip result is the same than the input
    fn assert_roundtrip(input: &str) {
        assert_formatted(input, input, true)
    }

    fn assert_roundtrip_noncompact(input: &str) {
        assert_formatted(input, input, false)
    }

    // Asserts the input is formatted to the expected output
    fn assert_formatted(expected: &str, input: &str, compact: bool) {
        let docs = YamlLoader::load_from_str(input).unwrap();
        let first_doc = &docs[0];

        let mut output = String::new();
        let mut emitter = YamlEmitter::new(&mut output);
        emitter.compact(compact);
        emitter.dump(first_doc).unwrap();

        assert_eq!(expected, output)
    }

    fn fixture_roundtrip(input: &str, expected: &str, record: bool) {
        let input = fs::read_to_string(input).expect("cannot read input fixture");
        let loaded = YamlLoader::load_from_str(&input).expect("cannot load input fixture");
        let mut actual = String::new();
        YamlEmitter::new(&mut actual).dump(&loaded[0]).unwrap();

        if record {
            fs::write(expected, actual).expect("cannot record fixture");
        } else {
            let expected = fs::read_to_string(expected).expect("cannot read expected fixture");
            assert_eq!(expected, actual);
        }
    }
}
