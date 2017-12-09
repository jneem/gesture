use euclid;
use std::f64;
use std::f64::consts::PI;
use std::ops::{Add, Sub};

pub struct Mm;
pub type Point = euclid::TypedVector2D<f64, Mm>;

/// Represents an angle.
///
/// This type doesn't differentiate between multiples of full rotations; that is,
/// an angle of 360 degrees is treated the same as an angle of 0 degrees.
#[derive(Clone, Copy, Debug)]
pub struct Angle {
    angle: f64,
}

impl Angle {
    /// Creates an `Angle` from radians.
    ///
    /// The result doesn't change if `r` changes by a multiple of `2π`.
    pub fn from_radians(r: f64) -> Angle {
        let shift = (r / (2.0 * PI)).floor();
        Angle {
            angle: r - 2.0 * PI * shift,
        }
    }

    /// Converts an `Angle` to radians.
    ///
    /// The result is guaranteed to be in the interval `[0, 2π)`.
    pub fn to_radians(&self) -> f64 {
        self.angle
    }

    /// Computes the "distance" from the angle to zero.
    ///
    /// If you convert the result of this to radians, it is guaranteed
    /// to be in the interval `[0, π]`.
    ///
    /// ```
    /// use libgestures::geom::Angle;
    /// assert_eq!(Angle::from_radians(std::f64::consts::PI).abs().to_radians(), std::f64::consts::PI);
    /// assert_eq!(Angle::from_radians(- std::f64::consts::PI / 2.0).abs().to_radians(), std::f64::consts::PI / 2.0);
    /// ```
    pub fn abs(&self) -> Angle {
        Angle {
            angle: (2.0 * PI - self.angle).min(self.angle),
        }
    }
}

impl Add<Angle> for Angle {
    type Output = Angle;

    fn add(self, other: Angle) -> Angle {
        Angle::from_radians(self.angle + other.angle)
    }
}

impl Sub<Angle> for Angle {
    type Output = Angle;

    fn sub(self, other: Angle) -> Angle {
        Angle::from_radians(self.angle - other.angle)
    }
}

