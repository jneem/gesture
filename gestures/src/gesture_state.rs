use euclid::vec2;
use input::event::touch::{TouchEvent, TouchEventPosition, TouchEventSlot, TouchEventTrait};
use Point;
use std::f64::consts::PI;

// We don't pay any attention to more than this many fingers.
const MAX_SLOTS: usize = 10;

#[derive(Clone, Debug)]
pub enum Gesture {
    Swipe { angle: f64 }
}

#[derive(Clone, Debug)]
pub struct Manager {
    frame: Frame,
    swipe: SwipeState,
    phase: Phase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Idle,
    Checking,
    Cancelled,
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            frame: Frame::new(),
            swipe: SwipeState::Idle,
            phase: Phase::Idle,
        }
    }

    pub fn update_phase(&mut self) -> Option<Gesture> {
        let mut gesture = None;

        self.phase = match self.phase {
            Phase::Idle => {
                if self.frame.touch_up && self.frame.cur.num_down > 0 {
                    Phase::Cancelled
                } else if self.frame.cur.num_down >= 3 {
                    // TODO: we should be a little more stringent here: we want to get three
                    // touches almost simultaneously
                    gesture = self.swipe.update(&self.frame);
                    self.swipe.phase()
                } else {
                    Phase::Idle
                }
            }
            Phase::Checking => {
                gesture = self.swipe.update(&self.frame);
                self.swipe.phase()
            }
            Phase::Cancelled => {
                if self.frame.cur.num_down == 0 {
                    Phase::Idle
                }
            }
        }
        gesture
    }

    pub fn update(&mut self, ev: &TouchEvent) -> Option<Gesture> {
        self.frame.update(ev);
        if let Some(&TouchEvent::Frame(_) = ev) {
            let ret self.update_phase();
            self.frame.last = self.frame.cur;
            ret
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub num_down: u8,
    pub down: [bool; MAX_SLOTS],
    pub pos: [Point; MAX_SLOTS],
}

impl Snapshot {
    pub fn new() -> Snapshot {
        Snapshot {
            num_down: 0,
            down: [false; MAX_SLOTS],
            pos: [vec2(0.0, 0.0); MAX_SLOTS],
        }
    }

    pub fn mean_pos(&self) -> Point {
        let sum: Point = (0..MAX_SLOTS)
            .filter(|i| self.down[*i])
            .map(|i| self.pos[i])
            .fold(vec2(0.0, 0.0), |a, b| {a + b});
        if self.num_down == 0 {
            vec2(0.0, 0.0)
        } else {
            sum / (self.num_down as f64)
        }
    }

    pub fn mean_dist(&self, other: &Snapshot) -> f64 {
        let sum: f64 = (0..MAX_SLOTS)
            .filter(|i| self.down[*i] && other.down[*i])
            .map(|i| (self.pos[i] - other.pos[i]).length())
            .sum();
        if self.num_down == 0 {
            0.0
        } else {
            sum / (self.num_down as f64)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub touch_down: bool,
    pub touch_up: bool,
    pub cur: Snapshot,
    pub last: Snapshot,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            touch_down: false,
            touch_up: false,
            cur: Snapshot::new(),
            last: Snapshot::new(),
        }
    }


    pub fn update(&mut self, ev: &TouchEvent) {
        match ev {
            &TouchEvent::Down(ref ev) => {
                let slot = ev.slot().unwrap_or(0) as usize;

                if slot >= MAX_SLOTS {
                    println!("not enough slots for {:?}", ev);
                    return;
                }
                if self.cur.down[slot] {
                    println!("down event, but the finger was already down?");
                    return;
                }

                self.touch_down = true;
                self.cur.down[slot] = true;
                self.cur.pos[slot] = vec2(ev.x(), ev.y());
                self.cur.num_down += 1;
            },
            &TouchEvent::Up(ref ev) => {
                let slot = ev.slot().unwrap_or(0) as usize;
                if !self.cur.down[slot] {
                    println!("up event, but the finger was already up?");
                    return;
                }

                self.touch_up = true;
                self.cur.down[slot] = false;
                self.cur.num_down -= 1;
            },
            &TouchEvent::Motion(ref ev) => {
                let slot = ev.slot().unwrap_or(0) as usize;
                self.cur.pos[slot] = vec2(ev.x(), ev.y());
            },
            &TouchEvent::Cancel(_) => {
                println!("what should I do with a cancel event?");
            },
            &TouchEvent::Frame(_) => {
                println!("new frame");
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum GestureProgress {
    Starting,
    Checking,
    Finishing,
    Success,
    Failure,
}

#[derive(Clone, Debug)]
pub enum SwipeState {
    Starting {
        init_pos: Point,
    },
    Swiping {
        init_pos: Point,
        angle: f64,
        distance: f64,
    },
    Finishing {
        angle: f64,
    },
    Idle,
}

const SWIPE_THRESHOLD_MM: f64 = 5.0;
const ANGLE_THRESHOLD_DEG: f64 = 10.0;
const DISTANCE_THRESHOLD_MM: f64 = 1.0;

impl SwipeState {
    pub fn phase(&self) -> Phase {
        match self {
            &SwipeState::Starting => Phase::Checking,
            &SwipeState::Swiping => Phase::Checking,
            &SwipeState::Finishing => Phase::Checking,
            &SwipeState::Idle => 
        }
    }

    pub fn update(&mut self, frame: &Frame) -> Option<Gesture> {
        *self = match *self {
            SwipeState::Starting { init_pos } => {
                if frame.touch_up {
                    println!("aborted: got touch up before we got going");
                    SwipeState::Idle
                } else if frame.touch_down {
                        // A new finger down will affect the mean position, so update it.
                    SwipeState::Starting {
                        init_pos: frame.cur.mean_pos()
                    }
                } else {
                    let pos = frame.cur.mean_pos();
                    let diff = pos - init_pos;
                    // TODO: check if the fingers have moved apart (while preserving the mean)
                    if diff.length() > SWIPE_THRESHOLD_MM {
                        println!("starting swipe: angle {:?}", diff.y.atan2(diff.x));
                        SwipeState::Swiping {
                            init_pos: init_pos,
                            angle: diff.y.atan2(diff.x),
                            distance: diff.length(),
                        }
                    } else {
                        self
                    }
                }
            }
            SwipeState::Swiping { init_pos, angle, distance } => {
                if frame.touch_up {
                    println!("fingers going up, switching to finishing");
                    SwipeState::Finishing { angle }
                } else if frame.touch_down {
                    println!("aborted: got touch down while swiping");
                    SwipeState::Idle
                } else {
                    let pos = frame.cur.mean_pos();
                    let diff = pos - init_pos;
                    let new_angle = diff.y.atan2(diff.x);
                    let new_distance = diff.length();
                    let angle_diff = angle - new_angle;
                    if angle_diff.abs() > ANGLE_THRESHOLD_DEG
                            && (angle_diff - 2*PI).abs() > ANGLE_THRESHOLD_DEG
                            && (angle_diff + 2*PI).abs() > ANGLE_THRESHOLD_DEG {
                        println!("aborted: too large a change to the initial angle");
                        SwipeState::Idle
                    } else if new_distance < distance - DISTANCE_THRESHOLD_MM {
                        println!("aborted: backtracked");
                        SwipeState::Idle
                    } else {
                        SwipeState::Swiping {
                            init_pos: init_pos,
                            angle: angle,
                            distance: distance.max(new_distance)
                        }
                    }
                }
            }
            SwipeState::Finishing { angle } => {
                if frame.touch_down {
                    println!("aborted: got touch down while finishing");
                    SwipeState::Idle
                } else if frame.cur.num_down == 0 {
                    println!("done!");
                    SwipeState::Done { angle }
                } else {
                    // TODO: check that we didn't move much since we started finishing
                    self
                }
            }
            SwipeState::Done { .. } => SwipeState::Idle,
            SwipeState::Idle => SwipeState::Starting { init_pos: frame.cur.mean_pos() }
        };
        None
    }
}
