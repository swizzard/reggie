pub mod alternatives;
pub mod char_set;
pub mod element;
pub mod flags;
pub mod groups;
pub mod pattern;
pub mod quantified;
pub mod quantifiers;

pub use alternatives::Alternatives;
pub use char_set::{CharClass, CharSet};
pub use element::{Element, Literal, ZeroWidthLiteral};
pub use flags::Flags;
pub use groups::Group;
pub use pattern::Pattern;
pub use quantifiers::Quantifier;

pub trait Component {
    fn as_string(&self) -> String;
    fn flags(&self) -> Flags;
    fn indexed(&self) -> bool;
    fn is_finite(&self) -> bool;
    fn min_match_len(&self) -> usize;
}
