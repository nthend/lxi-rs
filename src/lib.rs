use std::io::prelude::*;
use std::io::{self, BufReader, BufWriter};
use std::net::{TcpStream};

pub struct LxiDevice {
    addr: (String, u16),
    stream: Option<LxiStream>,
}

struct LxiStream {
    inp: BufReader<TcpStream>,
    out: BufWriter<TcpStream>,
}

impl LxiDevice {
    pub fn new(addr: (String, u16)) -> Self {
        Self { addr, stream: None }
    }

    pub fn address(&self) -> (&str, u16) {
        (self.addr.0.as_str(), self.addr.1)
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub fn connect(&mut self) -> io::Result<()> {
        if self.is_connected() {
            return Err(io::ErrorKind::AlreadyExists.into())
        }
        let stream = TcpStream::connect(self.address())?;
        let inp = BufReader::new(stream.try_clone()?);
        let out = BufWriter::new(stream);
        self.stream = Some(LxiStream { inp, out });
        Ok(())
    }

    pub fn disconnect(&mut self) -> io::Result<()> {
        if !self.is_connected() {
            return Err(io::ErrorKind::NotConnected.into())
        }
        self.stream = None;
        Ok(())
    }

    pub fn reconnect(&mut self) -> io::Result<()> {
        self.disconnect()
        .and_then(|()| self.connect())
    }

    pub fn send(&mut self, text: &str) -> io::Result<()> {
        self.stream.as_mut().ok_or(io::ErrorKind::NotConnected.into())
        .and_then(|stream| stream.send(text))
    }

    pub fn receive(&mut self) -> io::Result<String> {
        self.stream.as_mut().ok_or(io::ErrorKind::NotConnected.into())
        .and_then(|stream| stream.receive())
    }

    pub fn request(&mut self, text: &str) -> io::Result<String> {
        self.stream.as_mut().ok_or(io::ErrorKind::NotConnected.into())
        .and_then(|stream| stream.request(text))
    }
}

impl LxiStream {
    fn send(&mut self, text: &str) -> io::Result<()> {
        self.out.write_all(text.as_bytes())
        .and_then(|()| self.out.write_all(b"\r\n"))
        .and_then(|()| self.out.flush())
    }

    fn receive(&mut self) -> io::Result<String> {
        let mut buf = Vec::new();
        self.inp.read_until(b'\n', &mut buf)
        .and_then(|_num| {
            let mut text = String::from_utf8_lossy(&buf).into_owned();
            remove_newline(&mut text);
            Ok(text)
        })
    }

    fn request(&mut self, text: &str) -> io::Result<String> {
        self.send(text)
        .and_then(|()| self.receive())
    }
}

fn remove_newline(text: &mut String) {
    match text.pop() {
        Some('\n') => match text.pop() {
            Some('\r') => (),
            Some(c) => text.push(c),
            None => (),
        },
        Some(c) => text.push(c),
        None => (),
    }
}

#[cfg(test)]
mod emul;

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;
    use std::time::{Duration};

    use emul::{Emulator};

    #[test]
    fn idn() {
        let e = Emulator::new(("localhost", 0)).unwrap();
        let p = e.address().unwrap().port();
        let e = e.run();

        thread::sleep(Duration::from_millis(100));

        {
            let mut d = LxiDevice::new((String::from("localhost"), p));
            d.connect().unwrap();
            assert_eq!(d.request(&"*IDN?").unwrap(), "Emulator");
        }

        e.join().unwrap().unwrap();
    }
}
