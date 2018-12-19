use coord_2d::{Coord, Size};

pub trait OutputWrap {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord>;
}

pub struct WrapNone;
pub struct WrapX;
pub struct WrapY;
pub struct WrapXY;

impl OutputWrap for WrapNone {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord> {
        if coord.is_valid(size) {
            Some(coord)
        } else {
            None
        }
    }
}

fn value_is_valid(value: i32, size: u32) -> bool {
    value >= 0 && (value as u32) < size
}

fn normalize_value(value: i32, size: u32) -> i32 {
    let value = value % size as i32;
    if value < 0 {
        value + size as i32
    } else {
        value
    }
}

impl OutputWrap for WrapX {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord> {
        if value_is_valid(coord.y, size.y()) {
            let x = normalize_value(coord.x, size.x());
            Some(Coord::new(x, coord.y))
        } else {
            None
        }
    }
}

impl OutputWrap for WrapXY {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord> {
        Some(coord.normalize(size))
    }
}

impl OutputWrap for WrapY {
    fn normalize_coord(coord: Coord, size: Size) -> Option<Coord> {
        if value_is_valid(coord.x, size.x()) {
            let y = normalize_value(coord.y, size.y());
            Some(Coord::new(coord.x, y))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn wraps() {
        assert_eq! {
            WrapNone::normalize_coord(Coord::new(2, 3), Size::new(4, 5)),
            Some(Coord::new(2, 3))
        };
        assert_eq! {
            WrapNone::normalize_coord(Coord::new(4, 3), Size::new(4, 5)),
            None,
        };
        assert_eq! {
            WrapX::normalize_coord(Coord::new(4, 3), Size::new(4, 5)),
            Some(Coord::new(0, 3)),
        };
        assert_eq! {
            WrapY::normalize_coord(Coord::new(4, 3), Size::new(4, 5)),
            None,
        };
        assert_eq! {
            WrapY::normalize_coord(Coord::new(2, 6), Size::new(4, 5)),
            Some(Coord::new(2, 1)),
        };
        assert_eq! {
            WrapXY::normalize_coord(Coord::new(2, 6), Size::new(4, 5)),
            Some(Coord::new(2, 1)),
        };
    }
}
