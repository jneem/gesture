use euclid::vec2;
use input::event::touch::{TouchEvent, TouchEventPosition, TouchEventSlot};
use geom::Point;

/// Summarizes the changes that took place in a `libinput` frame.
///
/// Libinput sends its events grouped together, in "frames". That is, it sends a bunch of input
/// events and then it sends a "frame" event. All of the input events that happened between
/// two frame events should be treated as though they happened simultaneously.
///
/// Since it would be tedious for all of the individual gesture recognizers to interpret frame
/// events themselves, this struct exists to summarize all of the changes that happened during
/// the most recent frame.
#[derive(Clone, Debug)]
pub struct Frame {
    /// Did a `TouchDown` event happen during the last frame?
    pub touch_down: bool,
    /// Did a `TouchUp` event happen during the last frame?
    pub touch_up: bool,
    /// What are the current positions of all the fingers?
    pub cur: Snapshot,
    /// What were the last positions of all the fingers?
    pub last: Snapshot,
}

impl Frame {
    /// Creates a new `Frame`.
    pub fn new() -> Frame {
        Frame {
            touch_down: false,
            touch_up: false,
            cur: Snapshot::new(),
            last: Snapshot::new(),
        }
    }

    /// Updates a `Frame` to account for a new `TouchEvent` that just happened.
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
            },
        }
    }

    /// Resets the `Frame` to get ready for the next frame.
    pub fn advance(&mut self) {
        self.last = self.cur;
        self.touch_up = false;
        self.touch_down = false;
    }
}

pub const MAX_SLOTS: usize = 10;

/// A `Snapshot` storesa snapshot of the state of the fingers.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    /// How many fingers are down?
    pub num_down: u8,
    /// Which of the indices in `pos` represent fingers that are down?
    pub down: [bool; MAX_SLOTS],
    /// What are the positions of the fingers that are down?
    pub pos: [Point; MAX_SLOTS],
}

impl Snapshot {
    /// Creates a new, empty, `Snapshot`.
    pub fn new() -> Snapshot {
        Snapshot {
            num_down: 0,
            down: [false; MAX_SLOTS],
            pos: [vec2(0.0, 0.0); MAX_SLOTS],
        }
    }

    /// Returns the arithmetic mean of all the fingers that are down.
    ///
    /// If there are no down fingers, returns zero.
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

    /// Returns the mean distance between the fingers that are down in both `self` and `other`.
    ///
    /// If there are no fingers that are down in both snapshots, returns zero.
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

    /// If there are fingers that are down in `other` but not `self`, copy them over.
    pub fn merge_from(&mut self, other: &Snapshot) {
        for i in 0..MAX_SLOTS {
            if other.down[i] && !self.down[i] {
                self.down[i] = true;
                self.pos[i] = other.pos[i];
                self.num_down += 1;
            }
        }
    }
}


