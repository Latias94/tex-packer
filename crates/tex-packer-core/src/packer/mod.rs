use crate::model::{Frame, Rect};

pub mod guillotine;
pub mod maxrects;
pub mod skyline;

/// A packer places rectangles into a page.
///
/// Implementations must ensure no overlaps and respect the configured border/padding.
/// `pack` may return `None` if the rectangle cannot be placed on the current page.
pub trait Packer<K> {
    fn can_pack(&self, rect: &Rect) -> bool;
    fn pack(&mut self, key: K, rect: &Rect) -> Option<Frame<K>>;
}
