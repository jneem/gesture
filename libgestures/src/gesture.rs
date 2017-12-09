use euclid::vec2;
use input::event::touch::TouchEvent;
use frame::{Frame, Snapshot};
use geom::{Angle, Point};
use std::fmt::Debug;


// TODO: move this out of libgestures
#[derive(Clone, Debug)]
pub enum Gesture {
    Swipe { angle: Angle }
}

/// The result of trying to recognize a gesture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GestureResult<T> {
    /// We need more input to decide whether the gesture succeeded.
    Continuing,
    /// The gesture is finished.
    Succeeded(T),
    /// The gesture was not recognized.
    Failed,
}

impl<T> GestureResult<T> {
    /// Changes the output of a `GestureResult` by applying a function to it.
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> GestureResult<U> {
        match self {
            GestureResult::Continuing => GestureResult::Continuing,
            GestureResult::Failed => GestureResult::Failed,
            GestureResult::Succeeded(t) => GestureResult::Succeeded(f(t)),
        }
    }
}

/// A `Recognizer` is the main trait involved in recognizing gestures.
///
/// TODO: more
pub trait Recognizer: Debug {
    type In;
    type Out;

    /// Initializes this `Recognizer`, given an input and the current frame.
    fn init(&mut self, input: Self::In, frame: &Frame);

    /// Updates the `Recognizer` with a new frame.
    ///
    /// Returns a `GestureResult` indicating whether the `Recognizer` has finished
    /// recognizing a gesture.
    fn update(&mut self, frame: &Frame) -> GestureResult<Self::Out>;

    /// Takes a closure and returns a `Recognizer` that recognizes exactly the same gesture as this
    /// one, but has a different output type.
    fn map<U, F: FnMut(Self::Out) -> U>(self, f: F) -> Map<Self, F> where Self: Sized {
        Map {
            rec: self,
            f: f,
        }
    }

    /// Composes this `Recognizer` with another one, to create a `Recognizer` that tries to
    /// recognize our gesture first, and then the other one.
    ///
    /// The `Out` type of `self` needs to match the `In` type of `other`, so that when the first
    /// gesture finishes, the second gesture can be initialized with the result of the first one.
    fn and_then<U, R: Recognizer<In=Self::Out, Out=U>>(self, other: R)
    -> Composition<Self, R> where Self: Sized {
        Composition::new(self, other)
    }

    /// Takes a `Filter` and returns a new `Recognizer` that succeeds if and only if the original
    /// `Recognizer` recognized something *and* the `Filter` didn't abort.
    fn constrain<F: Filter>(self, f: F)
    -> Constraint<Self, F> where Self: Sized {
        Constraint { rec: self, fil: f }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterResult {
    Passed,
    Failed,
}

pub trait Filter: Debug {
    fn init(&mut self, frame: &Frame);
    fn update(&mut self, frame: &Frame) -> FilterResult;
}

#[derive(Clone)]
pub struct Map<Rec, F> {
    rec: Rec,
    f: F,
}

impl<Rec: Debug, F> Debug for Map<Rec, F> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Map<{:?}, f>", self.rec)
    }
}

impl<In, Out, Rec, T, F> Recognizer for Map<Rec, F>
where
Rec: Recognizer<In=In, Out=T>,
F: FnMut(T) -> Out,
{
    type In = In;
    type Out = Out;

    fn init(&mut self, input: Self::In, frame: &Frame) {
        self.rec.init(input, frame);
    }

    fn update(&mut self, frame: &Frame) -> GestureResult<Self::Out> {
        self.rec.update(frame).map(&mut self.f)
    }
}

#[derive(Clone, Debug)]
pub struct NFingers {
    n: u8,
}

impl NFingers {
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

    fn update(&mut self, frame: &Frame) -> GestureResult<()> {
        if frame.touch_up || frame.cur.num_down > self.n {
            GestureResult::Failed
        } else if frame.cur.num_down == self.n {
            GestureResult::Succeeded(())
        } else {
            GestureResult::Continuing
        }
    }
}

#[derive(Clone, Debug)]
pub struct Composition<Rec1, Rec2> {
    rec1: Rec1,
    rec2: Rec2,
    on_rec2: bool,
}

impl<T, Rec1: Recognizer<Out=T>, Rec2: Recognizer<In=T>> Composition<Rec1, Rec2> {
    pub fn new(rec1: Rec1, rec2: Rec2) -> Composition<Rec1, Rec2> {
        Composition {
            rec1: rec1,
            rec2: rec2,
            on_rec2: false,
        }
    }
}

impl<T, Rec1: Recognizer<Out=T>, Rec2: Recognizer<In=T>> Recognizer for Composition<Rec1, Rec2> {
    type In = Rec1::In;
    type Out = Rec2::Out;

    fn init(&mut self, input: Self::In, frame: &Frame) {
        self.rec1.init(input, frame);
        self.on_rec2 = false;
    }

    fn update(&mut self, frame: &Frame) -> GestureResult<Self::Out> {
        if self.on_rec2 {
            self.rec2.update(frame)
        } else {
            match self.rec1.update(frame) {
                GestureResult::Failed => GestureResult::Failed,
                GestureResult::Continuing => GestureResult::Continuing,
                GestureResult::Succeeded(x) => {
                    self.on_rec2 = true;
                    self.rec2.init(x, frame);
                    GestureResult::Continuing
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Constraint<Rec, Fil> {
    rec: Rec,
    fil: Fil,
}

impl<Rec: Recognizer, Fil: Filter> Recognizer for Constraint<Rec, Fil> {
    type In = Rec::In;
    type Out = Rec::Out;

    fn init(&mut self, input: Self::In, frame: &Frame) {
        self.rec.init(input, frame);
        self.fil.init(frame);
    }

    fn update(&mut self, frame: &Frame) -> GestureResult<Self::Out> {
        if self.fil.update(frame) == FilterResult::Failed {
            GestureResult::Failed
        } else {
            self.rec.update(frame)
        }
    }
}


#[derive(Clone, Debug)]
pub struct InitialAngle {
    threshold: f64,
    init_pos: Point,
}

impl InitialAngle {
    pub fn new() -> InitialAngle {
        InitialAngle {
            threshold: 1.0,
            init_pos: vec2(0.0, 0.0),
        }
    }

    pub fn with_threshold(mm: f64) -> InitialAngle {
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

    fn update(&mut self, frame: &Frame) -> GestureResult<(Point, Angle)> {
        if frame.touch_up || frame.touch_down {
            GestureResult::Failed
        } else {
            let pos = frame.cur.mean_pos();
            let diff = pos - self.init_pos;
            // TODO: check if the fingers have moved apart (while preserving the mean)
            if diff.length() > self.threshold {
                GestureResult::Succeeded((self.init_pos, Angle::from_radians(diff.y.atan2(diff.x))))
            } else {
                GestureResult::Continuing
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum StraightSwipeReason {
    ChangedAngle,
    LiftedFinger,
}

#[derive(Clone, Copy, Debug)]
pub struct StraightSwipeOutcome {
    reason: StraightSwipeReason,
    init_pos: Point,
    final_pos: Point,
    angle: Angle,
}

#[derive(Clone, Debug)]
pub struct StraightSwipe {
    init_pos: Point,
    min_length: f64,
    angle: Angle,
    angle_tolerance: f64,
}

impl StraightSwipe {
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
        self.angle = init.1;
    }

    fn update(&mut self, frame: &Frame) -> GestureResult<Self::Out> {
        if frame.touch_down {
            GestureResult::Failed
        } else if frame.touch_up {
            let diff = frame.cur.mean_pos() - self.init_pos;
            if diff.length() > self.min_length {
                GestureResult::Succeeded(self.outcome(StraightSwipeReason::LiftedFinger, frame))
            } else {
                GestureResult::Failed
            }
        } else {
            let diff = frame.cur.mean_pos() - frame.last.mean_pos();
            let angle = Angle::from_radians(diff.y.atan2(diff.x));
            if (angle - self.angle).abs().to_radians() > self.angle_tolerance {
                GestureResult::Failed
            } else {
                GestureResult::Continuing
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NoMovement {
    threshold: f64,
    init_pos: Snapshot,
}

impl NoMovement {
    pub fn new() -> NoMovement {
        NoMovement {
            threshold: 1.0,
            init_pos: Snapshot::new(),
        }
    }
}

impl Filter for NoMovement {
    fn init(&mut self, frame: &Frame) {
        self.init_pos = frame.cur;
    }

    fn update(&mut self, frame: &Frame) -> FilterResult {
        if frame.cur.mean_dist(&self.init_pos) > self.threshold {
            FilterResult::Failed
        } else {
            self.init_pos.merge_from(&frame.cur);
            FilterResult::Passed
        }
    }
}

#[derive(Debug)]
pub struct Manager {
    active: Vec<Box<Recognizer<In=(), Out=Gesture>>>,
    inactive: Vec<Box<Recognizer<In=(), Out=Gesture>>>,
    buf: Vec<Box<Recognizer<In=(), Out=Gesture>>>,
    frame: Frame,
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            active: vec![],
            inactive: vec![],
            buf: vec![],
            frame: Frame::new(),
        }
    }

    pub fn push<R: Recognizer<In=(), Out=Gesture> + 'static>(&mut self, r: R) {
        self.active.push(Box::new(r));
    }

    pub fn update(&mut self, ev: &TouchEvent) -> Option<Gesture> {
        self.frame.update(ev);
        if let &TouchEvent::Frame(_) = ev {
            if self.frame.last.num_down == 0 && self.frame.cur.num_down > 0 {
                for r in &mut self.inactive {
                    r.init((), &self.frame);
                }
                self.active.extend(self.inactive.drain(..));
            }

            let mut ret = None;
            for mut rec in self.active.drain(..) {
                match rec.update(&self.frame) {
                    GestureResult::Continuing => self.buf.push(rec),
                    GestureResult::Failed => self.inactive.push(rec),
                    GestureResult::Succeeded(g) => {
                        ret = Some(g);
                        self.inactive.push(rec);
                    }
                }
            }
            ::std::mem::swap(&mut self.buf, &mut self.active);
            self.frame.advance();
            ret
        } else {
            None
        }
    }
}

