use coord_2d::*;
use grid_2d::coord_system::{CoordSystem, XThenY, XThenYIter};
use grid_2d::*;
use std::hash::{Hash, Hasher};

pub fn new<'a, T, S: CoordSystem + Clone>(
    grid: &'a Grid<T, S>,
    offset: Coord,
    size: Size,
) -> TiledGridSlice<'a, T, S> {
    TiledGridSlice { grid, offset, size }
}

#[derive(Clone)]
pub struct TiledGridSlice<'a, T: 'a, S: 'a + CoordSystem + Clone = XThenY> {
    grid: &'a Grid<T, S>,
    offset: Coord,
    size: Size,
}

pub struct TiledGridSliceIter<'a, T: 'a, S: 'a + CoordSystem> {
    grid: &'a Grid<T, S>,
    coord_iter: XThenYIter,
    offset: Coord,
}

impl<'a, T, S> Iterator for TiledGridSliceIter<'a, T, S>
where
    S: CoordSystem + Clone,
{
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.coord_iter
            .next()
            .map(|coord| self.grid.get_tiled(self.offset + coord))
    }
}

impl<'a, T, S> TiledGridSlice<'a, T, S>
where
    S: CoordSystem + Clone,
{
    pub fn offset(&self) -> Coord {
        self.offset
    }
    pub fn get(&self, coord: Coord) -> Option<&T> {
        if coord.is_valid(self.size) {
            Some(self.grid.get_tiled(self.offset + coord))
        } else {
            None
        }
    }
    pub fn iter(&self) -> TiledGridSliceIter<T, S> {
        TiledGridSliceIter {
            grid: self.grid,
            offset: self.offset,
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
    use std::collections::HashSet;
    #[test]
    fn tiling() {
        let grid = Grid::new_fn(Size::new(4, 4), |coord| coord);
        let slice = new(&grid, Coord::new(-1, -1), Size::new(2, 2));
        let value = *slice.get(Coord::new(0, 1)).unwrap();
        assert_eq!(value, Coord::new(3, 0));
    }
    #[test]
    fn tiled_grid_slice_hash() {
        let mut grid = Grid::new_fn(Size::new(4, 4), |_| 0);
        *grid.get_mut(Coord::new(3, 3)).unwrap() = 1;
        let size = Size::new(2, 2);
        let a = new(&grid, Coord::new(0, 0), size);
        let b = new(&grid, Coord::new(2, 2), size);
        let c = new(&grid, Coord::new(0, 2), size);
        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        set.insert(c);
        assert_eq!(set.len(), 2);
    }
}
