use coord_2d::{Coord, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Orientation {
    /// ##.
    /// ...
    /// ...
    Original,
    /// ..#
    /// ..#
    /// ...
    Clockwise90,
    /// ...
    /// ...
    /// .##
    Clockwise180,
    /// ...
    /// #..
    /// #..
    Clockwise270,
    /// #..
    /// #..
    /// ...
    DiagonallyFlipped,
    /// .##
    /// ...
    /// ...
    DiagonallyFlippedClockwise90,
    /// ...
    /// ..#
    /// ..#
    DiagonallyFlippedClockwise180,
    /// ...
    /// ...
    /// ##.
    DiagonallyFlippedClockwise270,
}

pub const NUM_ORIENTATIONS: usize = 8;

use self::Orientation::*;
pub const ALL: [Orientation; NUM_ORIENTATIONS] = [
    Original,
    Clockwise90,
    Clockwise180,
    Clockwise270,
    DiagonallyFlipped,
    DiagonallyFlippedClockwise90,
    DiagonallyFlippedClockwise180,
    DiagonallyFlippedClockwise270,
];

impl Orientation {
    pub(crate) fn transform_coord(self, size: Size, coord: Coord) -> Coord {
        match self {
            Original => coord,
            Clockwise90 => Coord::new(coord.y, size.x() as i32 - 1 - coord.x),
            Clockwise180 => {
                Coord::new(size.x() as i32 - 1 - coord.x, size.y() as i32 - 1 - coord.y)
            }
            Clockwise270 => Coord::new(size.y() as i32 - 1 - coord.y, coord.x),
            DiagonallyFlipped => Coord::new(coord.y, coord.x),
            DiagonallyFlippedClockwise90 => {
                Coord::new(size.x() as i32 - 1 - coord.x, coord.y)
            }
            DiagonallyFlippedClockwise180 => {
                Coord::new(size.y() as i32 - 1 - coord.y, size.x() as i32 - 1 - coord.x)
            }
            DiagonallyFlippedClockwise270 => {
                Coord::new(coord.x, size.y() as i32 - 1 - coord.y)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrientationTable<T> {
    table: [Option<T>; NUM_ORIENTATIONS],
}

impl<T> OrientationTable<T> {
    pub fn new() -> Self {
        Self {
            table: [None, None, None, None, None, None, None, None],
        }
    }
    pub fn get(&self, orientation: Orientation) -> Option<&T> {
        self.table[orientation as usize].as_ref()
    }
    pub fn get_mut(&mut self, orientation: Orientation) -> Option<&mut T> {
        self.table[orientation as usize].as_mut()
    }
    pub fn insert(&mut self, orientation: Orientation, value: T) {
        self.table[orientation as usize] = Some(value);
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.table.iter().filter_map(|t| t.as_ref())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn checks() {
        let size = Size::new(3, 3);
        assert_eq!(
            Orientation::Clockwise90.transform_coord(size, Coord::new(1, 2)),
            Coord::new(2, 1)
        );
        assert_eq!(
            Orientation::Clockwise90.transform_coord(size, Coord::new(0, 0)),
            Coord::new(0, 2)
        );
    }
}
