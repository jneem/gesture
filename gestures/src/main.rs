#[macro_use] extern crate chan;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

extern crate app_dirs;
extern crate chan_signal;
extern crate env_logger;
extern crate input;
extern crate libc;
extern crate libgestures;
extern crate libudev_sys;
extern crate serde;
extern crate toml;

use chan_signal::Signal;
use input::event::Event;
use libgestures::Recognizer;
use libgestures::geom::Direction;
use libgestures::gestures::compound::direction_swipe;
use libgestures::manager::Manager;
use std::collections::HashSet;

const APP_INFO: app_dirs::AppInfo = app_dirs::AppInfo {
    name: "gestures",
    author: "Joe Neeman",
};

mod config;
mod libinput;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Gesture {
    Swipe {
        num_fingers: u8,
        direction: Direction,
    },
}

fn main() {
    if let Err(e) = env_logger::init() {
        println!("failed to initialize logging: {:?}", e);
    }

    let config = config::open_config();
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);
    let mut input = libinput::input().unwrap();
    let mut man = Manager::new();

    let mut fingers = HashSet::new();
    for gesture in config.bindings.keys() {
        match gesture {
            &Gesture::Swipe { num_fingers, .. } => fingers.insert(num_fingers),
        };
    }
    for &num_fingers in &fingers {
        man.push(direction_swipe(num_fingers).map_outcome(move |direction| Gesture::Swipe { num_fingers, direction }));
    }

    // Consume the initial events.
    input.libinput.dispatch().unwrap();
    while let Some(_) = input.libinput.next() {
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
                            if let Some(action) = config.bindings.get(&g) {
                                action.run();
                            }
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

