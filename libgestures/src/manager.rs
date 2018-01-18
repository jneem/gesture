use input::event::touch::TouchEvent;

use frame::Frame;
use {Recognizer, RecResult};

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

