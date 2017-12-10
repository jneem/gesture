#[macro_use] extern crate chan;

extern crate chan_signal;
extern crate input;
extern crate libc;
extern crate libgestures;
extern crate libudev_sys;

use chan_signal::Signal;
use input::event::Event;
use libgestures::Recognizer;
use libgestures::geom::Angle;
use libgestures::gesture::{InitialAngle, Manager, NoMovement, NFingers};

mod libinput;

#[derive(Clone, Debug)]
pub enum Gesture {
    Swipe { angle: Angle }
}

fn main() {
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);
    let mut input = libinput::input().unwrap();
    let mut man = Manager::new();
    let rec = NFingers::new(3)
        .constrain(NoMovement::new())
        .and_then(InitialAngle::new())
        .map(|(_, angle)| Gesture::Swipe { angle });
    man.push(rec);

    input.libinput.dispatch().unwrap();
    while let Some(event) = input.libinput.next() {
        println!("got initial event: {:?}", event);
    }

    let poll = input.poll;
    loop {
        chan_select! {
            poll.recv() => {
                input.libinput.dispatch().unwrap();
                while let Some(event) = input.libinput.next() {
                    if let Event::Touch(ev) = event {
                        if let Some(g) = man.update(&ev) {
                            println!("got gesture {:?}", g);
                        }
                    }
                }
            },
            signal.recv() -> _ => {
                break;
            },
        }
    }
}

