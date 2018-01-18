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
#[derive(Clone, Copy, Debug, PartialEq)]
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

    /// Creates an `Angle` from a number of degrees.
    ///
    /// The result doesn't change if `d` changes by a multiple of `360`.
    pub fn from_degrees(d: f64) -> Angle {
        Angle::from_radians(d * PI / 180.0)
    }

    /// Converts an `Angle` to radians.
    ///
    /// The result is guaranteed to be in the interval `[0, 2π)`.
    pub fn to_radians(&self) -> f64 {
        self.angle
    }

    /// Converts an `Angle` to degrees.
    ///
    /// The result is guaranteed to be in the interval `[0, 360)`.
    pub fn to_degrees(&self) -> f64 {
        self.angle * 180.0 / PI
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
    pub fn abs(&self) -> UAngle {
        UAngle {
            angle: (2.0 * PI - self.angle).min(self.angle),
        }
    }

    /// Computes the convex combination of two angles.
    ///
    /// `lambda` must be between `0.0` and `1.0`; the return value is effectively `(1-lambda)*self
    /// + lambda * other`. The interpolation of two angles will always be *between* the two, in the
    /// sense that if we think of the two angles as dividing the circle into two parts, then the
    /// interpolation will always be in the smaller part.
    ///
    /// ```
    /// use libgestures::geom::Angle;
    /// use std::f64::consts::PI;
    ///
    /// // The angles pi/4 and 7*pi/4 are close to zero, so interpolating between them (in either
    /// // order) will give zero.
    /// assert_eq!(
    ///     Angle::from_radians(0.0),
    ///     Angle::from_radians(7.0 * PI / 4.0).interpolate(Angle::from_radians(PI / 4.0), 0.5)
    /// );
    /// assert_eq!(
    ///     Angle::from_radians(0.0),
    ///     Angle::from_radians(PI / 4.0).interpolate(Angle::from_radians(7.0 * PI / 4.0), 0.5)
    /// );
    ///
    /// // The angles 3*pi/4 and 5*pi/4 are close to pi, so interpolating between them will give
    /// // pi.
    /// assert_eq!(
    ///     Angle::from_radians(PI),
    ///     Angle::from_radians(3.0 * PI / 4.0).interpolate(Angle::from_radians(5.0 * PI / 4.0), 0.5)
    /// );
    /// assert_eq!(
    ///     Angle::from_radians(PI),
    ///     Angle::from_radians(5.0 * PI / 4.0).interpolate(Angle::from_radians(3.0 * PI / 4.0), 0.5)
    /// );
    /// ```
    pub fn interpolate(&self, other: Angle, lambda: f64) -> Angle {
        assert!(0.0 <= lambda && lambda <= 1.0);

        let mut my_angle = self.angle;
        let mut other_angle = other.angle;

        // If necessary, adjust one of the angles so that they're within PI of each other.
        if (my_angle - other_angle).abs() > PI {
            if my_angle < other_angle {
                my_angle += 2.0 * PI;
            } else {
                other_angle += 2.0 * PI;
            }
        }

        let ret_angle = (1.0 - lambda) * my_angle + lambda * other_angle;
        Angle::from_radians(ret_angle)
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

/// An unsized angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UAngle {
    angle: f64
}

impl UAngle {
    pub fn from_radians(radians: f64) -> UAngle {
        assert!(radians >= 0.0);
        UAngle {
            angle: Angle::from_radians(radians).to_radians(),
        }
    }

    pub fn to_radians(&self) -> f64 {
        self.angle
    }

    pub fn from_degrees(degrees: f64) -> UAngle {
        assert!(degrees >= 0.0);
        UAngle::from_radians(degrees * PI / 180.0)
    }

    pub fn to_degrees(&self) -> f64 {
        self.angle * 180.0 / PI
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn from_angle(angle: Angle, threshold: UAngle) -> Option<Direction> {
        let t = threshold.to_radians();
        assert!(t <= PI / 4.0);
        let a = angle.to_radians();
        let right = 0.0;
        let up = PI / 2.0;
        let left = PI;
        let down = 1.5 * PI;

        if (0.0..=(right + t)).contains(a) {
            Some(Direction::Right)
        } else if ((up - t)..=(up + t)).contains(a) {
            Some(Direction::Up)
        } else if ((left - t)..=(left + t)).contains(a) {
            Some(Direction::Left)
        } else if ((down - t)..=(down + t)).contains(a) {
            Some(Direction::Down)
        } else if ((2.0 * PI - t)..=(2.0 * PI)).contains(a) {
            Some(Direction::Right)
        } else {
            None
        }
    }

    pub fn to_angle(&self) -> Angle {
        use self::Direction::*;

        match *self {
            Right => Angle::from_degrees(0.0),
            Up => Angle::from_degrees(90.0),
            Left => Angle::from_degrees(180.0),
            Down => Angle::from_degrees(270.0),
        }
    }
}
