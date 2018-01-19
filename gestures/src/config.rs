use app_dirs::{app_root, AppDataType};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::process;
use toml;

use { APP_INFO, Direction, Gesture };

fn parse_swipe(mut s: &[&str]) -> Option<Gesture> {
    if s.is_empty() {
        return None;
    }

    let mut num_fingers = 3;
    if let Ok(n) = s[0].parse::<u8>() {
        num_fingers = n;
        s = &s[1..];
    }

    if s.len() != 1 {
        return None;
    }
    let direction = match s[0] {
        "up" => Direction::Up,
        "down" => Direction::Down,
        "left" => Direction::Left,
        "right" => Direction::Right,
        _ => return None,
    };
    Some(Gesture::Swipe { num_fingers, direction })
}

fn parse_gesture(s: &str) -> Option<Gesture> {
    let parts = s.split_whitespace().collect::<Vec<_>>();
    match parts[0] {
        "swipe" => {
            parse_swipe(&parts[1..])
        },
        _ => {
            error!("unable to parse gesture {:?}", s);
            None
        },
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ConfigParsed {
    bindings: Vec<BindingParsed>,
}

impl ConfigParsed {
    fn to_config(self) -> Result<Config, String> {
        let mut ret = Config {
            bindings: HashMap::new(),
        };

        for b in self.bindings {
            let (gesture, action) = b.to_binding()?;
            if ret.bindings.insert(gesture, action).is_some() {
                return Err("duplicate binding".to_owned());
            }
        }

        Ok(ret)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct BindingParsed {
    gesture: String,
    command: String,
    args: Vec<String>,
}

impl BindingParsed {
    fn to_binding(self) -> Result<(Gesture, Action), String> {
        let g = parse_gesture(&self.gesture).ok_or("Error parsing gesture in config file")?;
        let action = Action::Command {
            command: self.command,
            args: self.args,
        };
        Ok((g, action))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub bindings: HashMap<Gesture, Action>
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    Command {
        command: String,
        args: Vec<String>,
    }
}

impl Action {
    pub fn run(&self) {
        match self {
            &Action::Command { ref command, ref args } => {
                let res = process::Command::new(command)
                    .args(args)
                    .spawn();
                if let Err(e) = res {
                    error!("failed to execute command {:?}: {}", command, e);
                }
            }
        }
    }
}

pub fn open_config() -> Config {
    let mut file_name = app_root(AppDataType::UserConfig, &APP_INFO).expect("couldn't open config directory") ;
    file_name.push("bindings.toml");

    let mut file = File::open(&file_name).expect("couldn't open config file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("unable to read config file");
    let c: ConfigParsed = toml::from_str(&contents).expect("unable to parse config file");
    c.to_config().unwrap()
}


