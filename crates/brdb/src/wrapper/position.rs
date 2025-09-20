use std::cmp::Ordering;

use crate::{CHUNK_HALF, CHUNK_SIZE, ChunkIndex, RelativePosition};

#[derive(Copy, Clone, Debug, Default, PartialOrd, Eq, PartialEq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub const ZERO: Self = Self::new(0, 0, 0);
    pub const ONE: Self = Self::new(1, 1, 1);
    pub const CHUNK_SIZE: Self = Self::new(CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE);
    pub const CHUNK_HALF: Self = Self::new(CHUNK_HALF, CHUNK_HALF, CHUNK_HALF);
    pub const X: Self = Self::new(1, 0, 0);
    pub const Y: Self = Self::new(0, 1, 0);
    pub const Z: Self = Self::new(0, 0, 1);
    pub const NORTH: Self = Self::new(0, -1, 0);
    pub const SOUTH: Self = Self::new(0, 1, 0);
    pub const EAST: Self = Self::new(1, 0, 0);
    pub const WEST: Self = Self::new(-1, 0, 0);
    pub const UP: Self = Self::new(0, 0, 1);
    pub const DOWN: Self = Self::new(0, 0, -1);
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub fn to_relative(self) -> (ChunkIndex, RelativePosition) {
        // Brick positions are from -1024 to 1023 in a chunk
        // A brick at (0, 0, 0) is positioned at -1024, -1024, -1024 in the chunk (0, 0, 0)
        (
            ChunkIndex {
                x: self.x.div_euclid(CHUNK_SIZE) as i16,
                y: self.y.div_euclid(CHUNK_SIZE) as i16,
                z: self.z.div_euclid(CHUNK_SIZE) as i16,
            },
            RelativePosition {
                x: (self.x.rem_euclid(CHUNK_SIZE) - CHUNK_HALF) as i16,
                y: (self.y.rem_euclid(CHUNK_SIZE) - CHUNK_HALF) as i16,
                z: (self.z.rem_euclid(CHUNK_SIZE) - CHUNK_HALF) as i16,
            },
        )
    }

    pub fn from_relative(chunk: ChunkIndex, pos: RelativePosition) -> Self {
        Position {
            x: chunk.x as i32 * CHUNK_SIZE + (CHUNK_SIZE / 2) + pos.x as i32,
            y: chunk.y as i32 * CHUNK_SIZE + (CHUNK_SIZE / 2) + pos.y as i32,
            z: chunk.z as i32 * CHUNK_SIZE + (CHUNK_SIZE / 2) + pos.z as i32,
        }
    }
}
impl std::ops::Neg for Position {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}
impl std::ops::Add for Position {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}
impl std::ops::AddAssign for Position {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}
impl std::ops::Sub for Position {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}
impl std::ops::SubAssign for Position {
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
    }
}
impl std::ops::Mul<i32> for Position {
    type Output = Self;

    fn mul(self, scalar: i32) -> Self::Output {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}
impl std::ops::MulAssign<i32> for Position {
    fn mul_assign(&mut self, scalar: i32) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
    }
}
impl std::ops::Div<i32> for Position {
    type Output = Self;

    fn div(self, scalar: i32) -> Self::Output {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}
impl std::ops::DivAssign<i32> for Position {
    fn div_assign(&mut self, scalar: i32) {
        self.x /= scalar;
        self.y /= scalar;
        self.z /= scalar;
    }
}

impl From<(i32, i32, i32)> for Position {
    fn from((x, y, z): (i32, i32, i32)) -> Self {
        Self { x, y, z }
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.z.cmp(&other.z) {
            Ordering::Equal => match self.y.cmp(&other.y) {
                Ordering::Equal => self.x.cmp(&other.x),
                ord => ord,
            },
            ord => ord,
        }
    }
}
