use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Read, Write};

fn main() -> Result<(), Box<Error>> {
    let mapper: HashMap<char, char> =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz".chars().zip(
        "ДВСDЁҒGНІЈКLМПОРQЯЅТЦЏШХЧZавсdёfgніјкlмпорqгѕтцѵшхчz".chars())
        .collect()
        ;

    let stdin = io::stdin();
    let stdout = io::stdout();

    let byte_input = stdin.lock().bytes();
    let text_input = decode_utf8(from_result_iterator(byte_input));
    // https://rust-lang-nursery.github.io/rust-clippy/v0.0.212/index.html#clone_on_copy
    let text_output =
        text_input.map(|c| *mapper.get(&c).unwrap_or(&c));
    //let text_output = "שלום עשח".chars().map(|c| (Ok(c) as Result<char, Box<Error>>)); // FIXME remove
    let byte_output = encode_utf8(text_output);

    connect(byte_output, stdout.lock())?;

    Ok(())
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

    fn iter(self) -> ToResultIterator<Self>
        where Self: Sized {
        ToResultIterator(self)
    }
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

pub enum Void {
    ImNotJokingPleaseNeverEverUseThisAtAll,
}

pub struct PlainIterator<I>(I);
pub fn from_plain_iterator<I>(iter: I) -> PlainIterator<I> {
    PlainIterator(iter)
}
impl<I> EIterator for PlainIterator<I>
    where I: Iterator {

    type Item = I::Item;
    type Error = Void;

    fn enext(&mut self) -> Step<Self::Item, Self::Error> {
        match self.0.next() {
            Some(x) => Step::Yield(x),
            None => Step::Done,
        }
    }
}

pub struct ResultIterator<I>(I);
pub fn from_result_iterator<I>(iter: I) -> ResultIterator<I> {
    ResultIterator(iter)
}
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

impl<I, E> EIterator for DecodeUtf8<I>
    where I: EIterator<Item=u8, Error=E>,
          E: Error {
    type Item = char;
    type Error = Box<Error>;

    fn enext(&mut self) -> Step<Self::Item, Self::Error> {
        let b = match self.iter.enext() {
            Step::Done => {
                if self.count == 0 {
                    return Step::Done;
                } else {
                    return Step::Error(From::from("Incomplete UTF8 codepoint"));
                }
            }
            Step::Error(e) => {
                return Step::Error(From::from(e.to_string())); // FIXME to_string is a hack
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

pub fn decode_utf8<I>(iter: I) -> DecodeUtf8<I> {
    DecodeUtf8 {
        iter,
        count: 0,
        res: 0,
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

pub fn encode_utf8<I>(iter: I) -> EncodeUtf8<I> {
    EncodeUtf8 {
        iter,
        buf: [0; 4],
        index: 4,
    }
}

pub fn connect<I, W, E>(iter: I, mut hout: W) -> Result<(), E>
    where I: EIterator<Item=u8, Error=E>,
          W: Write,
          E: From<io::Error> {

    const SIZE: usize = 4096;
    let mut buf = [0; SIZE];
    let mut i = 0;

    for next in iter.iter() {
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
