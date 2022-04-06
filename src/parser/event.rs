use crate::scanner::TScalarStyle;
use crate::scanner::TokenType;

pub type AnchorID = usize;

/// `Event` is used with the low-level event base parsing API,
/// see `EventReceiver` trait.
#[derive(Clone, PartialEq, Debug, Eq)]
pub enum Event {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias(AnchorID),
    /// Value, style, anchor_id, tag
    Scalar(String, TScalarStyle, AnchorID, Option<TokenType>),
    SequenceStart(AnchorID),
    SequenceEnd,
    MappingStart(AnchorID),
    MappingEnd,
}

pub fn empty_scalar() -> Event {
    // a null scalar
    Event::Scalar("~".to_owned(), TScalarStyle::Plain, 0, None)
}

pub fn empty_scalar_with_anchor(anchor: usize, tag: Option<TokenType>) -> Event {
    Event::Scalar("".to_owned(), TScalarStyle::Plain, anchor, tag)
}
