pub use self::error::EmitError;
use self::funcs::escape_str;
use self::funcs::need_quotes;
use crate::yaml::Hash;
use crate::yaml::Yaml;
use std::fmt;

mod error;
mod funcs;

pub struct YamlEmitter<'a> {
    writer: &'a mut dyn fmt::Write,
    best_indent: usize,
    compact: bool,

    level: isize,
}

pub type EmitResult = Result<(), EmitError>;

impl<'a> YamlEmitter<'a> {
    pub fn new(writer: &'a mut dyn fmt::Write) -> YamlEmitter {
        YamlEmitter {
            writer,
            best_indent: 2,
            compact: true,
            level: -1,
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

    pub fn dump(&mut self, doc: &Yaml) -> EmitResult {
        writeln!(self.writer, "---")?;
        self.level = -1;
        self.emit_node(doc)
    }

    fn emit_node(&mut self, node: &Yaml) -> EmitResult {
        match *node {
            Yaml::Array(ref v) => self.emit_array(v),
            Yaml::Hash(ref v) => self.emit_hash(v),
            Yaml::String(ref v) => {
                if need_quotes(v) {
                    escape_str(self.writer, v)?;
                } else {
                    write!(self.writer, "{}", v)?;
                }
                Ok(())
            }
            Yaml::Boolean(v) => {
                match v {
                    true => write!(self.writer, "true")?,
                    false => write!(self.writer, "false")?,
                }
                Ok(())
            }
            Yaml::Integer(v) => {
                write!(self.writer, "{}", v)?;
                Ok(())
            }
            Yaml::Real(ref v) => {
                write!(self.writer, "{}", v)?;
                Ok(())
            }
            Yaml::Null | Yaml::BadValue => {
                write!(self.writer, "~")?;
                Ok(())
            }
            Yaml::Alias(_) => Ok(()),
        }
    }

    fn emit_array(&mut self, arr: &[Yaml]) -> EmitResult {
        if arr.is_empty() {
            write!(self.writer, "[]")?;
            return Ok(());
        }

        self.level += 1;
        for (idx, entry) in arr.iter().enumerate() {
            if idx > 0 {
                self.emit_line_begin()?;
            }
            write!(self.writer, "-")?;
            self.emit_value(true, entry)?;
        }
        self.level -= 1;
        Ok(())
    }

    fn emit_hash(&mut self, hash: &Hash) -> EmitResult {
        if hash.is_empty() {
            self.writer.write_str("{}")?;
            return Ok(());
        }

        self.level += 1;
        for (idx, (key, value)) in hash.iter().enumerate() {
            if idx > 0 {
                self.emit_line_begin()?;
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
        }
        self.level -= 1;
        Ok(())
    }

    /// Emit a yaml as a hash or array value: i.e., which should appear
    /// following a ":" or "-", either after a space, or on a new line.
    /// If `inline` is true, then the preceding characters are distinct
    /// and short enough to respect the compact flag.
    fn emit_value(&mut self, inline: bool, value: &Yaml) -> EmitResult {
        match *value {
            Yaml::Array(ref arr) => {
                if (inline && self.compact) || arr.is_empty() {
                    write!(self.writer, " ")?;
                } else {
                    writeln!(self.writer)?;
                    self.level += 1;
                    self.emit_indent()?;
                    self.level -= 1;
                }
                self.emit_array(arr)
            }
            Yaml::Hash(ref hash) => {
                if (inline && self.compact) || hash.is_empty() {
                    write!(self.writer, " ")?;
                } else {
                    writeln!(self.writer)?;
                    self.level += 1;
                    self.emit_indent()?;
                    self.level -= 1;
                }
                self.emit_hash(hash)
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

    fn emit_indent(&mut self) -> EmitResult {
        if self.level <= 0 {
            return Ok(());
        }
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

    #[test]
    fn test_emit_simple() {
        let s = "
# comment
a0 bb: val
a1:
    b1: 4
    b2: d
a2: 4 # i'm comment
a3: [1, 2, 3]
a4:
    - [a1, a2]
    - 2
";

        let docs = YamlLoader::load_from_str(&s).unwrap();
        let doc = &docs[0];
        let mut writer = String::new();
        {
            let mut emitter = YamlEmitter::new(&mut writer);
            emitter.dump(doc).unwrap();
        }
        println!("original:\n{}", s);
        println!("emitted:\n{}", writer);
        let docs_new = match YamlLoader::load_from_str(&writer) {
            Ok(y) => y,
            Err(e) => panic!("{}", e),
        };
        let doc_new = &docs_new[0];

        assert_eq!(doc, doc_new);
    }

    #[test]
    fn test_emit_complex() {
        let s = r#"
cataloge:
  product: &coffee   { name: Coffee,    price: 2.5  ,  unit: 1l  }
  product: &cookies  { name: Cookies!,  price: 3.40 ,  unit: 400g}

products:
  *coffee:
    amount: 4
  *cookies:
    amount: 4
  [1,2,3,4]:
    array key
  2.4:
    real key
  true:
    bool key
  {}:
    empty hash key
            "#;
        let docs = YamlLoader::load_from_str(&s).unwrap();
        let doc = &docs[0];
        let mut writer = String::new();
        {
            let mut emitter = YamlEmitter::new(&mut writer);
            emitter.dump(doc).unwrap();
        }
        let docs_new = match YamlLoader::load_from_str(&writer) {
            Ok(y) => y,
            Err(e) => panic!("{}", e),
        };
        let doc_new = &docs_new[0];
        assert_eq!(doc, doc_new);
    }

    #[test]
    fn test_emit_avoid_quotes() {
        let s = r#"---
a7: 你好
boolean: "true"
boolean2: "false"
date: 2014-12-31
empty_string: ""
empty_string1: " "
empty_string2: "    a"
empty_string3: "    a "
exp: "12e7"
field: ":"
field2: "{"
field3: "\\"
field4: "\n"
field5: "can't avoid quote"
float: "2.6"
int: "4"
nullable: "null"
nullable2: "~"
products:
  "*coffee":
    amount: 4
  "*cookies":
    amount: 4
  ".milk":
    amount: 1
  "2.4": real key
  "[1,2,3,4]": array key
  "true": bool key
  "{}": empty hash key
x: test
y: avoid quoting here
z: string with spaces"#;

        let docs = YamlLoader::load_from_str(&s).unwrap();
        let doc = &docs[0];
        let mut writer = String::new();
        {
            let mut emitter = YamlEmitter::new(&mut writer);
            emitter.dump(doc).unwrap();
        }

        assert_eq!(s, writer, "actual:\n\n{}\n", writer);
    }

    #[test]
    fn emit_quoted_bools() {
        let input = r#"---
string0: yes
string1: no
string2: "true"
string3: "false"
string4: "~"
null0: ~
[true, false]: real_bools
[True, TRUE, False, FALSE, y,Y,yes,Yes,YES,n,N,no,No,NO,on,On,ON,off,Off,OFF]: false_bools
bool0: true
bool1: false"#;
        let expected = r#"---
string0: "yes"
string1: "no"
string2: "true"
string3: "false"
string4: "~"
null0: ~
? - true
  - false
: real_bools
? - "True"
  - "TRUE"
  - "False"
  - "FALSE"
  - y
  - Y
  - "yes"
  - "Yes"
  - "YES"
  - n
  - N
  - "no"
  - "No"
  - "NO"
  - "on"
  - "On"
  - "ON"
  - "off"
  - "Off"
  - "OFF"
: false_bools
bool0: true
bool1: false"#;

        let docs = YamlLoader::load_from_str(&input).unwrap();
        let doc = &docs[0];
        let mut writer = String::new();
        {
            let mut emitter = YamlEmitter::new(&mut writer);
            emitter.dump(doc).unwrap();
        }

        assert_eq!(
            expected, writer,
            "expected:\n{}\nactual:\n{}\n",
            expected, writer
        );
    }

    #[test]
    fn test_empty_and_nested() {
        test_empty_and_nested_flag(false)
    }

    #[test]
    fn test_empty_and_nested_compact() {
        test_empty_and_nested_flag(true)
    }

    fn test_empty_and_nested_flag(compact: bool) {
        let s = if compact {
            r#"---
a:
  b:
    c: hello
  d: {}
e:
  - f
  - g
  - h: []"#
        } else {
            r#"---
a:
  b:
    c: hello
  d: {}
e:
  - f
  - g
  -
    h: []"#
        };

        let docs = YamlLoader::load_from_str(&s).unwrap();
        let doc = &docs[0];
        let mut writer = String::new();
        {
            let mut emitter = YamlEmitter::new(&mut writer);
            emitter.compact(compact);
            emitter.dump(doc).unwrap();
        }

        assert_eq!(s, writer);
    }

    #[test]
    fn test_nested_arrays() {
        let s = r#"---
a:
  - b
  - - c
    - d
    - - e
      - f"#;

        let docs = YamlLoader::load_from_str(&s).unwrap();
        let doc = &docs[0];
        let mut writer = String::new();
        {
            let mut emitter = YamlEmitter::new(&mut writer);
            emitter.dump(doc).unwrap();
        }
        println!("original:\n{}", s);
        println!("emitted:\n{}", writer);

        assert_eq!(s, writer);
    }

    #[test]
    fn test_deeply_nested_arrays() {
        let s = r#"---
a:
  - b
  - - c
    - d
    - - e
      - - f
      - - e"#;

        let docs = YamlLoader::load_from_str(&s).unwrap();
        let doc = &docs[0];
        let mut writer = String::new();
        {
            let mut emitter = YamlEmitter::new(&mut writer);
            emitter.dump(doc).unwrap();
        }
        println!("original:\n{}", s);
        println!("emitted:\n{}", writer);

        assert_eq!(s, writer);
    }

    #[test]
    fn test_nested_hashes() {
        let s = r#"---
a:
  b:
    c:
      d:
        e: f"#;

        let docs = YamlLoader::load_from_str(&s).unwrap();
        let doc = &docs[0];
        let mut writer = String::new();
        {
            let mut emitter = YamlEmitter::new(&mut writer);
            emitter.dump(doc).unwrap();
        }
        println!("original:\n{}", s);
        println!("emitted:\n{}", writer);

        assert_eq!(s, writer);
    }
}
