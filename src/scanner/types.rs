use super::Marker;

#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum TEncoding {
    Utf8,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum TScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

#[derive(Clone, PartialEq, Debug, Eq)]
pub enum TokenType {
    StreamStart(TEncoding),
    StreamEnd,
    /// major, minor
    VersionDirective(u32, u32),
    /// handle, prefix
    TagDirective(String, String),
    DocumentStart,
    DocumentEnd,
    BlockSequenceStart,
    BlockMappingStart,
    BlockEnd,
    FlowSequenceStart,
    FlowSequenceEnd,
    FlowMappingStart,
    FlowMappingEnd,
    BlockEntry,
    FlowEntry,
    Key,
    Value,
    Alias(String),
    Anchor(String),
    /// handle, suffix
    Tag(String, String),
    Scalar(TScalarStyle, String),
    Comment(String),
}

#[derive(Clone, PartialEq, Debug, Eq)]
pub struct Token(pub Marker, pub TokenType);

#[derive(Clone, PartialEq, Debug, Eq)]
pub struct SimpleKey {
    pub possible: bool,
    pub required: bool,
    pub token_number: usize,
    pub mark: Marker,
}

impl SimpleKey {
    pub fn new(mark: Marker) -> SimpleKey {
        SimpleKey {
            possible: false,
            required: false,
            token_number: 0,
            mark,
        }
    }
}
