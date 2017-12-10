use euclid::vec2;
use input::event::touch::TouchEvent;
use frame::{Frame, Snapshot};
use geom::{Angle, Point};
use recognizer::{Filter, FilterResult, Recognizer, RecResult};


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

    fn update(&mut self, frame: &Frame) -> RecResult<()> {
        if frame.touch_up || frame.cur.num_down > self.n {
            RecResult::Failed
        } else if frame.cur.num_down == self.n {
            RecResult::Succeeded(())
        } else {
            RecResult::Continuing
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

    fn update(&mut self, frame: &Frame) -> RecResult<(Point, Angle)> {
        if frame.touch_up || frame.touch_down {
            RecResult::Failed
        } else {
            let pos = frame.cur.mean_pos();
            let diff = pos - self.init_pos;
            // TODO: check if the fingers have moved apart (while preserving the mean)
            if diff.length() > self.threshold {
                RecResult::Succeeded((self.init_pos, Angle::from_radians(diff.y.atan2(diff.x))))
            } else {
                RecResult::Continuing
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
    pub reason: StraightSwipeReason,
    pub init_pos: Point,
    pub final_pos: Point,
    pub angle: Angle,
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

    fn update(&mut self, frame: &Frame) -> RecResult<Self::Out> {
        if frame.touch_down {
            RecResult::Failed
        } else if frame.touch_up {
            let diff = frame.cur.mean_pos() - self.init_pos;
            if diff.length() > self.min_length {
                RecResult::Succeeded(self.outcome(StraightSwipeReason::LiftedFinger, frame))
            } else {
                RecResult::Failed
            }
        } else {
            let diff = frame.cur.mean_pos() - frame.last.mean_pos();
            let angle = Angle::from_radians(diff.y.atan2(diff.x));
            if (angle - self.angle).abs().to_radians() > self.angle_tolerance {
                RecResult::Failed
            } else {
                RecResult::Continuing
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
pub struct Manager<T> {
    active: Vec<Box<Recognizer<In=(), Out=T>>>,
    inactive: Vec<Box<Recognizer<In=(), Out=T>>>,
    buf: Vec<Box<Recognizer<In=(), Out=T>>>,
    frame: Frame,
}

impl<T> Manager<T> {
    pub fn new() -> Manager<T> {
        Manager {
            active: vec![],
            inactive: vec![],
            buf: vec![],
            frame: Frame::new(),
        }
    }

    pub fn push<R: Recognizer<In=(), Out=T> + 'static>(&mut self, r: R) {
        self.active.push(Box::new(r));
    }

    pub fn update(&mut self, ev: &TouchEvent) -> Option<T> {
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
                    RecResult::Continuing => self.buf.push(rec),
                    RecResult::Failed => self.inactive.push(rec),
                    RecResult::Succeeded(g) => {
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

