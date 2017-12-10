use frame::Frame;
use std::fmt::Debug;

/// The result of trying to recognize a gesture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecResult<T> {
    /// We need more input to decide whether the gesture succeeded.
    Continuing,
    /// The gesture is finished.
    Succeeded(T),
    /// The gesture was not recognized.
    Failed,
}

impl<T> RecResult<T> {
    /// Changes the output of a `RecResult` by applying a function to it.
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> RecResult<U> {
        match self {
            RecResult::Continuing => RecResult::Continuing,
            RecResult::Failed => RecResult::Failed,
            RecResult::Succeeded(t) => RecResult::Succeeded(f(t)),
        }
    }
}

/// A `Recognizer` is the main trait involved in recognizing gestures.
///
/// TODO: more documentation, and examples
pub trait Recognizer: Debug {
    type In;
    type Out;

    /// Initializes this `Recognizer`, given an input and the current frame.
    fn init(&mut self, input: Self::In, frame: &Frame);

    /// Updates the `Recognizer` with a new frame.
    ///
    /// Returns a `RecResult` indicating whether the `Recognizer` has finished
    /// recognizing a gesture.
    fn update(&mut self, frame: &Frame) -> RecResult<Self::Out>;

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

/// A recognizer that maps the output value by applying a function.
///
/// This struct is usually created by the [map](trait.Recognizer.html#method.map) method on
/// [Recognizer](trait.Recognizer.html). See that method for more.
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

    fn update(&mut self, frame: &Frame) -> RecResult<Self::Out> {
        self.rec.update(frame).map(&mut self.f)
    }
}

/// A recognizer that recognizes one gesture and then another.
///
/// This struct is usually created by the [and_then](trait.Recognizer.html#method.and_then) method
/// on [Recognizer](trait.Recognizer.html). See that method for more.
#[derive(Clone, Debug)]
pub struct Composition<Rec1, Rec2> {
    rec1: Rec1,
    rec2: Rec2,
    on_rec2: bool,
}

impl<T, Rec1: Recognizer<Out=T>, Rec2: Recognizer<In=T>> Composition<Rec1, Rec2> {
    fn new(rec1: Rec1, rec2: Rec2) -> Composition<Rec1, Rec2> {
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

    fn update(&mut self, frame: &Frame) -> RecResult<Self::Out> {
        if self.on_rec2 {
            self.rec2.update(frame)
        } else {
            match self.rec1.update(frame) {
                RecResult::Failed => RecResult::Failed,
                RecResult::Continuing => RecResult::Continuing,
                RecResult::Succeeded(x) => {
                    self.on_rec2 = true;
                    self.rec2.init(x, frame);
                    RecResult::Continuing
                }
            }
        }
    }
}

/// The result of a [Filter](trait.Filter.html).
///
/// This is basically just a boolean, but with more descriptive names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterResult {
    Passed,
    Failed,
}

/// A `Filter` provides a way to filter out bad gestures.
///
/// `Filter` is a bit like a [`Recognizer`](trait.Recognizer.html) in that it receives frames of
/// input one by one and decides what to make of them. The difference is that a `Recognizer`
/// actually produces an interesting output, while a `Filter` just waits around until it fails.
///
/// The main use of a `Filter` is to pass it to the
/// [`constrain`](trait.Recognizer.html#method.constrain) method of `Recognizer`. See the
/// documentation on that method for more information and examples.
pub trait Filter: Debug {
    fn init(&mut self, frame: &Frame);
    fn update(&mut self, frame: &Frame) -> FilterResult;
}

/// A recognizer that recognizes the same gestures as `Rec`, but fails if `Fil` tells it to.
///
/// This struct is usually created by the
/// [`constrain`](trait.Recognizer.html#method.constrain) method of `Recognizer`. See the
/// documentation on that method for more information and examples.
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

    fn update(&mut self, frame: &Frame) -> RecResult<Self::Out> {
        if self.fil.update(frame) == FilterResult::Failed {
            RecResult::Failed
        } else {
            self.rec.update(frame)
        }
    }
}

