use crate::orientation::Orientation;
use coord_2d::*;
use grid_2d::*;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct TiledGridSlice<'a, T: 'a> {
    grid: &'a Grid<T>,
    offset: Coord,
    size: Size,
    orientation: Orientation,
}

pub struct TiledGridSliceIter<'a, T: 'a> {
    grid: &'a TiledGridSlice<'a, T>,
    coord_iter: CoordIter,
}

impl<'a, T> Iterator for TiledGridSliceIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.coord_iter
            .next()
            .map(|coord| self.grid.get_valid(coord))
    }
}

impl<'a, T> TiledGridSlice<'a, T> {
    pub fn new(
        grid: &'a Grid<T>,
        offset: Coord,
        size: Size,
        orientation: Orientation,
    ) -> Self {
        TiledGridSlice {
            grid,
            offset,
            size,
            orientation,
        }
    }
    pub fn size(&self) -> Size {
        self.size
    }
    fn get_valid(&self, coord: Coord) -> &'a T {
        let transformed_coord = self.orientation.transform_coord(self.size, coord);
        self.grid.get_tiled(self.offset + transformed_coord)
    }
    pub fn get_checked(&self, coord: Coord) -> &'a T {
        if coord.is_valid(self.size) {
            self.get_valid(coord)
        } else {
            panic!("coord is out of bounds");
        }
    }
    pub fn offset(&self) -> Coord {
        self.offset
    }
    pub fn iter(&self) -> TiledGridSliceIter<T> {
        TiledGridSliceIter {
            grid: self,
            coord_iter: CoordIter::new(self.size),
        }
    }
}

impl<'a, T: Hash> Hash for TiledGridSlice<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for value in self.iter() {
            value.hash(state);
        }
    }
}

impl<'a, T: PartialEq> PartialEq for TiledGridSlice<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.iter().zip(other.iter()).all(|(s, o)| s.eq(o))
    }
}
impl<'a, T: Eq> Eq for TiledGridSlice<'a, T> {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::orientation::Orientation;
    use coord_2d::{Coord, Size};
    use std::collections::HashSet;

    #[test]
    fn tiling() {
        let grid = Grid::new_fn(Size::new(4, 4), |coord| coord);
        let slice = TiledGridSlice::new(
            &grid,
            Coord::new(-1, -1),
            Size::new(2, 2),
            Orientation::Original,
        );
        let value = *slice.get_valid(Coord::new(0, 1));
        assert_eq!(value, Coord::new(3, 0));
    }
    #[test]
    fn tiled_grid_slice_hash() {
        let mut grid = Grid::new_fn(Size::new(4, 4), |_| 0);
        *grid.get_mut(Coord::new(1, 3)).unwrap() = 1;
        let size = Size::new(2, 2);
        let a = TiledGridSlice::new(&grid, Coord::new(0, 0), size, Orientation::Original);
        let b = TiledGridSlice::new(&grid, Coord::new(2, 2), size, Orientation::Original);
        let c = TiledGridSlice::new(&grid, Coord::new(0, 2), size, Orientation::Original);
        let d =
            TiledGridSlice::new(&grid, Coord::new(1, 2), size, Orientation::Clockwise270);
        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        set.insert(c);
        set.insert(d);
        assert_eq!(set.len(), 2);
    }
}
