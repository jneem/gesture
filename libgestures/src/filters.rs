use {Filter, FilterResult};
use frame::{Frame, Snapshot};

/// A filter that fails if a finger moves too much.
///
/// Fingers are allowed to go up and down, but they are not allowed to move once they are down.
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
			debug!("NoMovement failed");
            FilterResult::Failed
        } else {
            self.init_pos.merge(&frame.cur);
            FilterResult::Passed
        }
    }
}

/// A filter that fails if a finger moves too much relative to the others.
///
/// That is, this filter allows the hand as a whole to move, but it should retain the same
/// basic "shape." Fingers are allowed to go up or down.
#[derive(Clone, Debug)]
pub struct NoRelativeMovement {
    threshold: f64,
    adaptivity: f64,
    init_rel_pos: Snapshot,
}

impl NoRelativeMovement {
    pub fn new() -> NoRelativeMovement {
        NoRelativeMovement {
            threshold: 5.0,
            adaptivity: 0.02,
            init_rel_pos: Snapshot::new(),
        }
    }
}

impl Filter for NoRelativeMovement {
    fn init(&mut self, frame: &Frame) {
        self.init_rel_pos = frame.cur;
        self.init_rel_pos -= frame.cur.mean_pos();
    }

    fn update(&mut self, frame: &Frame) -> FilterResult {
        // If fingers went up or down, adjust the expected relative positions to account for the
        // fact that the mean position was shifted by the new fingers.
        if frame.touch_down || frame.touch_up {
            let mean_diff = frame.cur.mean_pos() - frame.last.mean_pos();
            let mean_diff_corrected = frame.cur.mean_pos_filtered(&frame.last)
                - frame.last.mean_pos_filtered(&frame.cur);
            // How much of the movement of the mean was due to the fingers going up or down, as
            // opposed to fingers moving?
            let offset = mean_diff - mean_diff_corrected;

            self.init_rel_pos -= offset;
        }

        let mut rel_pos = frame.cur;
        rel_pos -= frame.cur.mean_pos();

        if frame.touch_down || frame.touch_up {
            self.init_rel_pos.merge(&rel_pos);
        }

        if rel_pos.mean_dist(&self.init_rel_pos) > self.threshold {
			debug!("NoRelativeMovement failed");
            FilterResult::Failed
        } else {
            let dist = (frame.cur.mean_pos() - frame.last.mean_pos()).length();
            let lambda = (dist * self.adaptivity).min(1.0);
            self.init_rel_pos.interpolate_to(&rel_pos, lambda);

            FilterResult::Passed
        }
    }
}

