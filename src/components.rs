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

#[derive(Debug)]
pub struct GroupIndices<'a> {
    pat: &'a Pattern,
    named: HashMap<String, &'a SubPattern>,
    indexed: Vec<&'a SubPattern>,
}

impl<'a> GroupIndices<'a> {
    fn new(pat: &'a Pattern) -> Self {
        let mut indexed = Vec::with_capacity(pat.sub_patterns.len());
        let mut named = HashMap::new();
        GroupIndices::collect_component_groups(&mut indexed, &mut named, &pat.sub_patterns);
        Self {
            pat,
            named,
            indexed,
        }
    }
    fn collect_component_groups(
        indexed: &mut Vec<&'a SubPattern>,
        named: &mut HashMap<String, &'a SubPattern>,
        cs: &'a Vec<SubPattern>,
    ) {
        for c in cs.iter() {
            match c {
                SubPattern::Group(Group::Group {
                    ext,
                    name,
                    components,
                    ..
                }) => {
                    if let Some(GroupExt::NonCapturing) = ext {
                        continue;
                    } else {
                        if let Some(name) = name {
                            named.insert(name.to_string(), c);
                        };
                        indexed.push(c);
                        GroupIndices::collect_component_groups(indexed, named, components);
                    }
                }
                _ => continue,
            }
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum GroupIndex {
    Numbered(usize),
    Named(String),
}

impl<T> From<T> for GroupIndex
where
    String: From<T>,
{
    fn from(value: T) -> Self {
        let value = String::from(value);
        if let Ok(num) = value.parse::<usize>() {
            Self::Numbered(num)
        } else {
            Self::Named(value)
        }
    }
}
