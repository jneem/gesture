use euclid::vec2;
use std;

use frame::Frame;
use geom::{Angle, Point};
use {Recognizer, RecResult};

/// A recognizer that detects when a certain number of fingers are down.
///
/// In order for this recognizer to succeed, the required number of fingers must come down *before*
/// any fingers go up.
#[derive(Clone, Debug)]
pub struct NFingers {
    n: u8,
}

impl NFingers {
    /// Creates a new recognizer that succeeds when `n` fingers come down.
    pub fn new(n: u8) -> NFingers {
        NFingers {
            n: n
        }
    }
}

impl Recognizer for NFingers {
    type In = ();
    type Out = ();

    fn init(&mut self, _: (), _: &Frame) {}

    fn update(&mut self, frame: &Frame) -> RecResult<()> {
        if frame.touch_up || frame.cur.num_down > self.n {
			debug!("NFingers failed");
            RecResult::Failed
        } else if frame.cur.num_down == self.n {
			debug!("NFingers succeeded");
            RecResult::Succeeded(())
        } else {
            RecResult::Continuing
        }
    }
}

/// A recognizer that succeeds when all the fingers have gone up.
///
/// This recognizer will fail if any fingers go down.
#[derive(Clone, Debug)]
pub struct FingersUp {
}

impl FingersUp {
	pub fn new() -> FingersUp {
		FingersUp { }
	}
}

impl Recognizer for FingersUp {
	type In = ();
	type Out = ();

	fn init(&mut self, _: (), _: &Frame) {}

	fn update(&mut self, frame: &Frame) -> RecResult<()> {
		if frame.touch_down {
			debug!("FingersUp failed");
			RecResult::Failed
		} else if frame.cur.num_down == 0 {
			debug!("FingersUp succeeded");
			RecResult::Succeeded(())
		} else {
			RecResult::Continuing
		}
	}
}

/// A recognizer that detects when the average finger position starts to move.
///
/// This recognizer will fail if fingers go up or come down. In particular, you probably want to 
/// ensure that this recognizer starts recognizing after some fingers are already down.
///
/// If this successfully recognizes something, it will return the average starting position of the
/// fingers and also the angle of the movement.
#[derive(Clone, Debug)]
pub struct InitialAngle {
    threshold: f64,
    init_pos: Point,
}

impl InitialAngle {
    /// Creates a new recognizer for detecting when the average finger position starts to move.
    pub fn new() -> InitialAngle {
        InitialAngle {
            threshold: 5.0,
            init_pos: vec2(0.0, 0.0),
        }
    }

    /// Creates a new recognizer for detecting when the average finger position starts to move,
    /// with a custom threshold for detecting movement.
    ///
    /// This recognizer will succeed once the average finger position has moved by the given number
    /// of millimeters.
    pub fn with_threshold_mm(mm: f64) -> InitialAngle {
        InitialAngle {
            threshold: mm,
            init_pos: vec2(0.0, 0.0),
        }
    }
}

impl Recognizer for InitialAngle {
    type In = ();
    type Out = (Point, Angle);

    fn init(&mut self, _: (), frame: &Frame) {
        self.init_pos = frame.cur.mean_pos();
    }

    fn update(&mut self, frame: &Frame) -> RecResult<(Point, Angle)> {
        if frame.touch_up || frame.touch_down {
			debug!("InitialAngle failed");
            RecResult::Failed
        } else {
            let pos = frame.cur.mean_pos();
            let diff = pos - self.init_pos;
            if diff.length() > self.threshold {
				debug!("InitialAngle succeeded: {:?} radians", (-diff.y).atan2(diff.x));
                RecResult::Succeeded((self.init_pos, Angle::from_radians((-diff.y).atan2(diff.x))))
            } else {
                RecResult::Continuing
            }
        }
    }
}

/// The possible reasons that a `StraightSwipe` finished recognizing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StraightSwipeReason {
	/// The straight swipe finished because it stopped being straight.
    ChangedAngle,
	/// The straight swipe finished because a finger was lifted.
    LiftedFinger,
}

/// The outcome of a successful `StraightSwipe`.
#[derive(Clone, Copy, Debug)]
pub struct StraightSwipeOutcome {
    /// The reason that the swipe finished recognizing.
    pub reason: StraightSwipeReason,

	/// The position where the swipe started. (More precisely, the arithmetic
	/// mean of the positions of all the fingers.)
    pub init_pos: Point,

	/// The position where the swipe finished. (More precisely, the arithmetic
	/// mean of the positions of all the fingers.)
    pub final_pos: Point,

	/// The angle of the swipe (measured counter-clockwise from the positive x
	/// axis). This is the *initial* angle of the swipe, and so it is not
	/// necessarily the same as the angle from `init_pos` to `final_pos`,
	/// although it should be pretty close.
    pub angle: Angle,
}

/// A recognizer that recognizes straight movements.
///
/// For this recognizer to succeed, the mean position of the fingers must move
/// in a straight line, and must continue to move in approximately the same
/// direction as the initial angle. We will fail to recognize if fingers go down.
/// 
/// If fingers go up, or if the angle changes, then we will succeed provided that
/// the mean position moved a reasonable distance in a straight line first.
#[derive(Clone, Debug)]
pub struct StraightSwipe {
    init_pos: Point,
    last_pos: Point,
    min_length: f64,
    step: f64,
    adaptivity: f64,
    angle: Angle,
    angle_tolerance: f64,
}

impl StraightSwipe {
    pub fn new() -> StraightSwipe {
        StraightSwipe {
            init_pos: vec2(0.0, 0.0),
            last_pos: vec2(0.0, 0.0),
            min_length: 10.0,
            step: 3.0,
            adaptivity: 0.01,
            angle: Angle::from_radians(0.0),
            angle_tolerance: 20.0 * std::f64::consts::PI / 180.0,
        }
    }

    pub fn min_length(self, length_mm: f64) -> StraightSwipe {
        StraightSwipe {
            min_length: length_mm,
            ..self
        }
    }

    pub fn adaptivity(self, adaptivity_per_mm: f64) -> StraightSwipe {
        StraightSwipe {
            adaptivity: adaptivity_per_mm,
            ..self
        }
    }

    fn outcome(&self, reason: StraightSwipeReason, frame: &Frame) -> StraightSwipeOutcome {
        StraightSwipeOutcome {
            reason: reason,
            init_pos: self.init_pos,
            final_pos: frame.cur.mean_pos(),
            angle: self.angle,
        }
    }
}

impl Recognizer for StraightSwipe {
    type In = (Point, Angle);
    type Out = StraightSwipeOutcome;

    fn init(&mut self, init: (Point, Angle), _: &Frame) {
        self.init_pos = init.0;
        self.last_pos = init.0;
        self.angle = init.1;
    }

    fn update(&mut self, frame: &Frame) -> RecResult<Self::Out> {
        if frame.touch_down {
			debug!("StraightSwipe failed because of a touch down");
            RecResult::Failed
        } else if frame.touch_up {
            let diff = frame.cur.mean_pos() - self.init_pos;
            if diff.length() > self.min_length {
				debug!("StraightSwipe succeeded after a lifted finger");
                RecResult::Succeeded(self.outcome(StraightSwipeReason::LiftedFinger, frame))
            } else {
				debug!("StraightSwipe failed because of a premature lift");
                RecResult::Failed
            }
        } else {
            let diff = frame.cur.mean_pos() - self.last_pos;
            if diff.length() >= self.step {
                self.last_pos = frame.cur.mean_pos();
                let angle = Angle::from_radians((-diff.y).atan2(diff.x));
                debug!("angle {:?}, self.angle {:?}", angle, self.angle);
                debug!("diff {:?}", (angle - self.angle).abs().to_radians());
                if (angle - self.angle).abs().to_radians() > self.angle_tolerance {
                    if (frame.cur.mean_pos() - self.init_pos).length() > self.min_length {
                        debug!("StraightSwipe succeeded after an angle change");
                        return RecResult::Succeeded(self.outcome(StraightSwipeReason::ChangedAngle, frame));
                    } else {
                        debug!("StraightSwipe failed because of a premature angle change");
                        return RecResult::Failed;
                    }
                }

                let lambda = (diff.length() * self.adaptivity).min(1.0);
                self.angle = self.angle.interpolate(angle, lambda);
            }
            RecResult::Continuing
        }
    }
}


