pub use self::error::EmitError;
use self::funcs::escape_str;
use self::funcs::need_quotes;
use crate::yaml::Hash;
use crate::yaml::Yaml;
use std::fmt;
use std::iter::Peekable;

mod error;
mod funcs;

macro_rules! debug_comment {
  ($msg:expr) => {
    comment_debug!($msg,);
  };
  ($msg:expr, $($opt:expr), *) => {
    println!("[DEBUG-COMMENT]");
    println!(" => {}", $msg);
    $(
      println!(" => {}: {:?}", stringify!($opt), $opt);
    )*
  };
}

macro_rules! debug_comment_disallowed {
  ($msg:expr) => {
    debug_comment_disallowed!($msg,);
  };
  ($msg:expr, $($opt:expr), *) => {
    debug_comment!($msg, $($opt)*);
    unreachable!("[DEBUG-COMMENT-DISALLOWED]");
  };
}

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
            Yaml::Comment(ref comment, inline) => {
                match inline {
                    true => write!(self.writer, " #{}", comment)?,
                    false => write!(self.writer, "#{}", comment)?,
                }
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
        let mut idx = -1;
        let mut iter = arr.iter().peekable();
        while let Some(entry) = iter.next() {
            idx += 1;
            if idx > 0 {
                self.emit_line_begin()?;
            }

            if entry.is_comment() {
                debug_comment!("emitting comment inside array (as entry)", entry);
                self.emit_node(entry)?;
                continue;
            }

            write!(self.writer, "-")?;
            self.emit_value(true, entry)?;

            if let Some(comment) = array_next_is_comment_inline(&mut iter) {
                debug_comment!(
                    "emitting comment inside array (inlined after value)",
                    comment
                );
                self.emit_node(comment)?;
            }
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
        let mut idx = -1;
        let mut iter = hash.iter().peekable();
        while let Some((key, value)) = iter.next() {
            idx += 1;
            if idx > 0 {
                self.emit_line_begin()?;
            }

            if key.is_comment() {
                debug_comment!("emitting comment inside hash (as key)", key);
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

            if let Some((key, _)) = hash_next_is_comment_inline(&mut iter) {
                debug_comment!("emitting comment inside hash (inlined after value)", key);
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
            Yaml::Comment(_, _) => {
                debug_comment_disallowed!("should never emit comment as value", value);
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
        for _ in 0..(self.level * self.best_indent as isize) {
            write!(self.writer, " ")?;
        }
        Ok(())
    }
}

fn hash_next_is_comment_inline<'a>(
    iter: &mut Peekable<impl Iterator<Item = (&'a Yaml, &'a Yaml)>>,
) -> Option<(&'a Yaml, &'a Yaml)> {
    if let Some((Yaml::Comment(_, inline), _)) = iter.peek() {
        if *inline {
            return iter.next();
        }
    }
    None
}

fn array_next_is_comment_inline<'a>(
    iter: &mut Peekable<impl Iterator<Item = &'a Yaml>>,
) -> Option<&'a Yaml> {
    if let Some(Yaml::Comment(_, inline)) = iter.peek() {
        if *inline {
            return iter.next();
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::YamlLoader;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_emit_simple() {
        let input = "
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

        let expected = "---
# comment
a0 bb: val
a1:
  b1: 4
  b2: d
a2: 4 # i'm comment
a3:
  - 1
  - 2
  - 3
a4:
  - - a1
    - a2
  - 2";
        assert_formatted(expected, input, true);
    }

    #[test]
    fn test_emit_complex() {
        let input = r#"
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
        let expected = r#"---
cataloge:
  product:
    name: Cookies!
    price: 3.40
    unit: 400g
products:
  ? name: Coffee
    price: 2.5
    unit: 1l
  : amount: 4
  ? name: Cookies!
    price: 3.40
    unit: 400g
  : amount: 4
  ? - 1
    - 2
    - 3
    - 4
  : array key
  2.4: real key
  true: bool key
  ? {}
  : empty hash key"#;

        assert_formatted(expected, input, true);
    }

    #[test]
    fn test_emit_avoid_quotes() {
        let input = r#"---
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

        assert_roundtrip(input);
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

        assert_formatted(expected, input, true);
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

    #[test]
    fn test_emitter_comments_yaml_helm_chart() {
        let input = r#"---
# Default values for example.
# This is a YAML-formatted file.
# Declare variables to be passed into your templates.
replicaCount: 1
image:
  repository: nginx
  tag: stable
  pullPolicy: IfNotPresent
imagePullSecrets: []
nameOverride: ""
fullnameOverride: ""
serviceAccount:
  # Specifies whether a service account should be created
  create: true
  # The name of the service account to use.
  # If not set and create is true, a name is generated using the fullname template
  name: some name
podSecurityContext: {}
# fsGroup: 2000
securityContext: {}
# capabilities:
#   drop:
#   - ALL
# readOnlyRootFilesystem: true
# runAsNonRoot: true
# runAsUser: 1000
service:
  type: ClusterIP
  port: 80
ingress:
  enabled: false
  annotations: {}
  # kubernetes.io/ingress.class: nginx
  # kubernetes.io/tls-acme: "true"
  hosts:
    - host: chart-example.local
      paths: []
      tls: []
  #  - secretName: chart-example-tls
  #    hosts:
  #      - chart-example.local
resources: {}
nodeSelector: {}
tolerations: []"#;

        assert_roundtrip(input);
    }

    #[test]
    fn test_emitter_comments_inline() {
        let input = r#"---
repos:
  # Is this supported?
  - repo: "https://github.com/rapidsai/frigate/"
    rev: v0.4.0 #  pre-commit autoupdate  - to keep the version up to date
    # and an in between keys comment here
    hooks:
      - id: frigate
        # Initial comment
        versions:
          - 1 # An inline comment
          # What?
          # Another comment
          # More comments...
          - 2
          - 3 # The last comment
          # And a confusing one too
  - repo: "https://github.com/gruntwork-io/pre-commit"
    rev: v0.1.12 #  pre-commit autoupdate  - to keep the version up to date
    hooks:
      - id: helmlint"#;

        assert_roundtrip(input);
    }

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
}
