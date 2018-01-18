#![feature(conservative_impl_trait, inclusive_range_syntax, range_contains)]

extern crate euclid;
extern crate input;

#[macro_use]
extern crate log;

pub mod filters;
pub mod frame;
pub mod geom;
pub mod gestures;
pub mod manager;
pub mod recognizer;

pub use recognizer::{Filter, FilterResult, Recognizer, RecResult};
