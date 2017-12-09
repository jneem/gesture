use chan;
use input;
use input::Libinput;
use libc;
use libc::{c_char, c_int, c_void};
use libudev_sys;

unsafe extern "C"
fn open_restricted(path: *const c_char, flags: c_int, _: *mut c_void) -> c_int {
    libc::open(path, flags)
}

unsafe extern "C"
fn close_restricted(fd: c_int, _: *mut c_void) {
    libc::close(fd);
}

static INTERFACE: input::LibinputInterface = input::LibinputInterface {
    open_restricted: Some(open_restricted),
    close_restricted: Some(close_restricted),
};

pub struct Input {
    pub libinput: Libinput,
    pub poll: chan::Receiver<()>,
}

fn init_libinput() -> Result<Libinput, ()> {
    unsafe {
        let udev = libudev_sys::udev_new();
        if udev.is_null() {
            return Err(());
        }

        // Pass in some nonsense userdata, because otherwise libinput segfaults on exit.
        let mut libinput = Libinput::new_from_udev::<()>(INTERFACE, Some(()), udev as *mut c_void);
        if let Err(_) = libinput.udev_assign_seat("seat0") {
            libudev_sys::udev_unref(udev);
            return Err(());
        }

        libudev_sys::udev_unref(udev);
        Ok(libinput)
    }
}

pub fn input() -> Result<Input, ()> {
    let libinput = init_libinput()?;
    let mut pollfd = libc::pollfd {
        fd: unsafe { libinput.fd() },
        events: libc::POLLIN,
        revents: 0,
    };

    let (send, recv) = chan::sync(0);
    ::std::thread::spawn(move || {
        while unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, -1) } >= 0 {
            send.send(());
        }
    });

    Ok(Input {
        libinput: libinput,
        poll: recv,
    })
}

