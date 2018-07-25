use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Read, Write};

fn main() -> Result<(), MyAppError> {
    let mapper: HashMap<char, char> =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz".chars().zip(
        "ДВСDЁҒGНІЈКLМПОРQЯЅТЦЏШХЧZавсdёfgніјкlмпорqгѕтцѵшхчz".chars())
        .collect()
        ;

    let stdin = io::stdin();
    let stdout = io::stdout();

    stdin
        .lock()
        .bytes()
        .eiter()
        .map_error(MyAppError::IOError)
        .decode_utf8()
        .map(|c| *mapper.get(&c).unwrap_or(&c))
        .encode_utf8()
        .write_to(stdout.lock())?;

    Ok(())
}

#[derive(Debug)]
pub enum MyAppError {
    IOError(std::io::Error),
    DecodeUtf8Error(DecodeUtf8Error),
}
impl From<std::io::Error> for MyAppError {
    fn from(e: std::io::Error) -> MyAppError {
        MyAppError::IOError(e)
    }
}
impl From<DecodeUtf8Error> for MyAppError {
    fn from(e: DecodeUtf8Error) -> MyAppError {
        MyAppError::DecodeUtf8Error(e)
    }
}
impl std::fmt::Display for MyAppError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MyAppError::IOError(e) => e.fmt(fmt),
            MyAppError::DecodeUtf8Error(e) => e.fmt(fmt),
        }
    }
}
impl Error for MyAppError {
    fn description(&self) -> &str {
        "MyAppError"
    }
    fn cause(&self) -> Option<&Error> {
        match self {
            MyAppError::IOError(e) => e.cause(),
            MyAppError::DecodeUtf8Error(e) => e.cause(),
        }
    }
}

pub enum Step<T, E> {
    Done,
    Yield(T),
    Skip,
    Error(E),
}

pub trait EIterator {
    type Item;
    type Error;

    fn enext(&mut self) -> Step<Self::Item, Self::Error>;

    fn step<F, B, E>(&mut self, mut f: F) -> Step<B, E>
        where F: FnMut(Self::Item) -> Step<B, E>,
              E: From<Self::Error> {
        match self.enext() {
            Step::Done => Step::Done,
            Step::Error(e) => Step::Error(From::from(e)),
            Step::Skip => Step::Skip,
            Step::Yield(x) => f(x),
        }
    }

    fn step_option<F, B, E>(&mut self, mut f: F) -> Step<B, E>
        where F: FnMut(Option<Self::Item>) -> Step<B, E>,
              E: From<Self::Error> {
        match self.enext() {
            Step::Done => f(None),
            Step::Error(e) => Step::Error(From::from(e)),
            Step::Skip => Step::Skip,
            Step::Yield(x) => f(Some(x)),
        }
    }

    fn map<B, F>(self, f: F) -> Map<Self, F>
        where Self: Sized, F: FnMut(Self::Item) -> B {
        Map {
            iter: self,
            func: f,
        }
    }

    fn map_error<E2, F>(self, f: F) -> MapError<Self, F>
        where Self: Sized, F: FnMut(Self::Error) -> E2 {
        MapError {
            iter: self,
            func: f,
        }
    }

    fn map_error_from<E2>(self) -> MapError<Self, fn(Self::Error) -> E2>
        where Self: Sized,
              E2: From<Self::Error> {
        self.map_error(From::from)
    }

    fn decode_utf8(self) -> DecodeUtf8<Self> where Self: Sized {
        DecodeUtf8 {
            iter: self,
            count: 0,
            res: 0,
        }
    }

    fn encode_utf8(self) -> EncodeUtf8<Self> where Self: Sized {
        EncodeUtf8 {
            iter: self,
            buf: [0; 4],
            index: 4,
        }
    }

    fn write_to<W: Write>(self, mut hout: W) -> Result<(), Self::Error>
        where Self: EIterator<Item=u8>,
              Self: Sized,
              Self::Error: From<io::Error> {

        const SIZE: usize = 4096;
        let mut buf = [0; SIZE];
        let mut i: usize = 0;

        for next in self.iter() {
            let b = next?;
            buf[i] = b;
            i += 1;

            if i == SIZE {
                hout.write_all(&buf)?;
                i = 0;
            }
        }

        if i > 0 {
            hout.write_all(&buf[..i])?;
        }

        Ok(())
    }

    fn iter(self) -> ToResultIterator<Self>
        where Self: Sized {
        ToResultIterator(self)
    }
}

pub trait ToEIter where Self: Sized {
    fn eiter(self) -> ResultIterator<Self> {
        ResultIterator(self)
    }
}

impl<I, T, E> ToEIter for I
    where I: Iterator<Item=Result<T, E>> {
}

pub struct Map<I, F> {
    iter: I,
    func: F,
}

impl<B, I: EIterator, F> EIterator for Map<I, F>
    where F: FnMut(I::Item) -> B {
    type Item = B;
    type Error = I::Error;

    fn enext(&mut self) -> Step<Self::Item, Self::Error> {
        let f = &mut self.func;
        self.iter.step(|x| Step::Yield(f(x)))
    }
}

pub struct MapError<I, F> {
    iter: I,
    func: F,
}

impl<E, I: EIterator, F> EIterator for MapError<I, F>
    where F: FnMut(I::Error) -> E {
    type Item = I::Item;
    type Error = E;

    fn enext(&mut self) -> Step<Self::Item, Self::Error> {
        match self.iter.enext() {
            Step::Done => Step::Done,
            Step::Skip => Step::Skip,
            Step::Error(e) => Step::Error((self.func)(e)),
            Step::Yield(x) => Step::Yield(x),
        }
    }
}

pub struct ToResultIterator<I>(I);

impl<I> Iterator for ToResultIterator<I>
    where I: EIterator {
    type Item = Result<I::Item, I::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.enext() {
                Step::Done => {
                    return None;
                }
                Step::Skip => (),
                Step::Error(e) => {
                    return Some(Err(e));
                }
                Step::Yield(x) => {
                    return Some(Ok(x));
                }
            }
        }
    }
}

pub struct ResultIterator<I>(I);
impl<I, T, E> EIterator for ResultIterator<I>
    where I: Iterator<Item=Result<T, E>> {

    type Item = T;
    type Error = E;

    fn enext(&mut self) -> Step<Self::Item, Self::Error> {
        match self.0.next() {
            Some(Ok(x)) => Step::Yield(x),
            Some(Err(e)) => Step::Error(e),
            None => Step::Done,
        }
    }
}

pub struct DecodeUtf8<I> {
    iter: I,
    count: u8,
    res: u32,
}

#[derive(Debug)]
pub enum DecodeUtf8Error {
    InvalidUtf8Codepoint,
}
impl std::fmt::Display for DecodeUtf8Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DecodeUtf8Error::InvalidUtf8Codepoint => {
                write!(fmt, "Invalid UTF8 codepoint")
            }
        }
    }
}
impl Error for DecodeUtf8Error {
    fn description(&self) -> &str {
        "UTF8 decode error"
    }
}

impl<I> EIterator for DecodeUtf8<I>
    where I: EIterator<Item=u8>,
          I::Error: From<DecodeUtf8Error>,
          I::Error: Error {
    type Item = char;
    type Error = I::Error;

    fn enext(&mut self) -> Step<Self::Item, Self::Error> {
        let b = match self.iter.enext() {
            Step::Done => {
                if self.count == 0 {
                    return Step::Done;
                } else {
                    return Step::Error(From::from(DecodeUtf8Error::InvalidUtf8Codepoint));
                }
            }
            Step::Error(e) => {
                return Step::Error(e);
            }
            Step::Skip => {
                return Step::Skip;
            }
            Step::Yield(b) => b,
        };

        if self.count == 0 {
            if b & 0b1000_0000 == 0 { // ASCII
                Step::Yield(unsafe { std::char::from_u32_unchecked(b.into()) })
            } else {
                self.count =
                    if b & 0b1110_0000 == 0b1100_0000 { // 2 bytes
                        self.res = u32::from(b & 0b0001_1111);
                        1
                    } else if b & 0b1111_0000 == 0b1110_0000 { // 3 bytes
                        self.res = u32::from(b & 0b0000_1111);
                        2
                    } else { // 4 bytes
                        assert!(b & 0b1111_1000 == 0b1111_0000);
                        self.res = u32::from(b & 0b0000_0111);
                        3
                    };
                Step::Skip
            }
        } else {
            self.count -= 1;
            self.res = (self.res << 6) | (u32::from(b) & 0b0011_1111);
            if self.count == 0 {
                Step::Yield(unsafe { std::char::from_u32_unchecked(self.res) })
            } else {
                Step::Skip
            }
        }
    }
}

pub struct EncodeUtf8<I> {
    pub iter: I,
    pub buf: [u8; 4],
    pub index: usize,
}

impl<I, E> EIterator for EncodeUtf8<I>
    where I: EIterator<Item=char, Error=E> {
    type Item = u8;
    type Error = E;

    fn enext(&mut self) -> Step<Self::Item, Self::Error> {
        if self.index < 4 {
            let res = self.buf[self.index];
            self.index += 1;
            if self.index < 4 && self.buf[self.index] == 0 {
                self.index = 4;
            }
            Step::Yield(res)
        } else {
            match self.iter.enext() {
                Step::Done => Step::Done,
                Step::Error(e) => Step::Error(e),
                Step::Skip => Step::Skip,
                Step::Yield(c) => {
                    let len = c.encode_utf8(&mut self.buf).len();
                    if len > 1 {
                        self.index = 1;
                        if len < 4 {
                            self.buf[len] = 0;
                        }
                    }
                    Step::Yield(self.buf[0])
                }
            }
        }
    }
}
