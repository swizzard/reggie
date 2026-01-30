use crate::components::flags::Flags;

pub trait AsComponent {
    fn as_string(&self) -> String;
    fn min_match_len(&self) -> usize;
    fn is_finite(&self) -> bool;
}

pub(crate) trait GroupLike {
    fn sub_components(&self) -> Vec<impl AsComponent>;
    fn indexed(&self) -> bool;
    fn flags(&self) -> Flags;
}
