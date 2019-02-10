use coord_2d::*;
use grid_2d::coord_system::{CoordSystem, XThenY, XThenYIter};
use grid_2d::*;
use orientation::Orientation;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct TiledGridSlice<'a, T: 'a, S: 'a + CoordSystem + Clone = XThenY> {
    grid: &'a Grid<T, S>,
    offset: Coord,
    size: Size,
    orientation: Orientation,
}

pub struct TiledGridSliceIter<'a, T: 'a, S: 'a + CoordSystem + Clone> {
    grid: &'a TiledGridSlice<'a, T, S>,
    coord_iter: XThenYIter,
}

impl<'a, T, S> Iterator for TiledGridSliceIter<'a, T, S>
where
    S: CoordSystem + Clone,
{
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.coord_iter
            .next()
            .map(|coord| self.grid.get_valid(coord))
    }
}

impl<'a, T, S> TiledGridSlice<'a, T, S>
where
    S: CoordSystem + Clone,
{
    pub fn new(
        grid: &'a Grid<T, S>,
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
    pub fn iter(&self) -> TiledGridSliceIter<T, S> {
        TiledGridSliceIter {
            grid: self,
            coord_iter: XThenYIter::from(self.size),
        }
    }
}

impl<'a, T: Hash, S> Hash for TiledGridSlice<'a, T, S>
where
    S: CoordSystem + Clone,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        for value in self.iter() {
            value.hash(state);
        }
    }
}

impl<'a, T: PartialEq, S> PartialEq for TiledGridSlice<'a, T, S>
where
    S: CoordSystem + Clone,
{
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.iter().zip(other.iter()).all(|(s, o)| s.eq(o))
    }
}
impl<'a, T: Eq, S> Eq for TiledGridSlice<'a, T, S> where S: CoordSystem + Clone {}

#[cfg(test)]
mod test {
    use super::*;
    use coord_2d::{Coord, Size};
    use orientation::Orientation;
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
        *grid.get_mut(Coord::new(3, 3)).unwrap() = 1;
        let size = Size::new(2, 2);
        let a = TiledGridSlice::new(&grid, Coord::new(0, 0), size, Orientation::Original);
        let b = TiledGridSlice::new(&grid, Coord::new(2, 2), size, Orientation::Original);
        let c = TiledGridSlice::new(&grid, Coord::new(0, 2), size, Orientation::Original);
        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        set.insert(c);
        assert_eq!(set.len(), 2);
    }
}
