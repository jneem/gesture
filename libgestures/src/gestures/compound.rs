use geom::{ Angle, Direction, Point, UAngle };
use filters::*;
use gestures::primitive::*;
use { Recognizer, RecResult };

pub struct SwipeResult {
    pub angle: Angle,
    pub length: f64,
}

pub fn angle_swipe() -> impl Recognizer<In=(), Out=Angle> {
	NFingers::new(3).constrain(NoMovement::new())
		.and_then(InitialAngle::new()
			.and_then(StraightSwipe::new())
			.constrain(NoRelativeMovement::new())
			.filter_outcome(|x| x.reason == StraightSwipeReason::LiftedFinger))
		.and_then(FingersUp::new()
			.split_input(|x: StraightSwipeOutcome| (x, ()))
			.map_outcome(|(x, _)| x.angle))
}

pub fn direction_swipe(num_fingers: u8) -> impl Recognizer<In=(), Out=Direction> {
    fn round_angle((pt, a): (Point, Angle)) -> RecResult<(Point, Direction)> {
        match Direction::from_angle(a, UAngle::from_degrees(25.0)) {
            Some(d) => RecResult::Succeeded((pt, d)),
            None => RecResult::Failed,
        }
    }

    // This is a Recognizer<In=(), Out=Direction>.
    let swipe =
        InitialAngle::new()
        .flat_map_outcome(round_angle)
        // So far, we have a Recognizer that returns (Point, Direction).
        .and_then(
            StraightSwipe::new()
            .adaptivity(0.0)
            .split_input(|(pt, d): (Point, Direction)| (d, (pt, d.to_angle())))
        )
        // So far, we have a Recognizer that returns (Direction, StraightSwipeOutcome).
        .constrain(NoRelativeMovement::new())
        .filter_outcome(|&(_, ref x)| x.reason == StraightSwipeReason::LiftedFinger)
        .map_outcome(|x| x.0);

    // This is a Recognizer<In=Direction, Out=()>.
    let up = FingersUp::new()
        .split_input(|d: Direction| (d, ()))
        .map_outcome(|(d, _)| d);

    NFingers::new(num_fingers).constrain(NoMovement::new())
        .and_then(swipe)
        .and_then(up)
}
