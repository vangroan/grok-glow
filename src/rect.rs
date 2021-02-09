use std::fmt::{self, Debug, Display};

/// General purpose 2D rectangle.
///
/// Contains a position and size.
#[derive(Debug, Clone, Copy)]
pub struct Rect<T: Debug + Copy> {
    pub pos: [T; 2],
    pub size: [T; 2],
}

impl<T> Display for Rect<T>
where
    T: Display + Debug + Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}, {}, {}, {}]",
            self.pos[0], self.pos[1], self.size[0], self.size[1]
        )
    }
}

impl<T> Rect<T>
where
    T: PartialOrd + Debug + Copy,
{
    /// Checks whether `other` can fit inside this rectangle.
    pub fn can_fit(&self, other: &Rect<T>) -> bool {
        other.pos[0] >= self.pos[0]
            && other.pos[1] >= self.pos[1]
            && other.size[0] <= self.size[0]
            && other.size[1] <= self.size[1]
    }
}
