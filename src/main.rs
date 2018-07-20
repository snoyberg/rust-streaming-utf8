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
    let text_input = decode_utf8(byte_input);
    // https://rust-lang-nursery.github.io/rust-clippy/v0.0.212/index.html#clone_on_copy
    let text_output =
        text_input.map(|res| res.map(|c| *mapper.get(&c).unwrap_or(&c)));
    //let text_output = "שלום עשח".chars().map(|c| (Ok(c) as Result<char, Box<Error>>)); // FIXME remove
    let byte_output = encode_utf8(text_output);

    connect(byte_output, stdout.lock())?;

    Ok(())
}

pub struct DecodeUtf8<I> {
    pub iter: I,
}

impl<I, E> DecodeUtf8<I>
    where I: Iterator<Item=Result<u8, E>>,
          E: Error {
    fn pop(&mut self) -> Result<u8, Box<Error>> {
        match self.iter.next() {
            None => Err(From::from("Incomplete UTF8 codepoint")),
            Some(Err(e)) => Err(From::from(e.to_string())), // FIXME Err(Box::new(e)),
            Some(Ok(b)) => Ok(b),
        }
    }
}

impl<I, E> Iterator for DecodeUtf8<I>
    where I: Iterator<Item=Result<u8, E>>,
          E: Error {
    type Item = Result<char, Box<Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        let b1 = match self.iter.next() {
            None => {
                return None;
            },
            Some(Err(e)) => {
                return Some(Err(From::from(e.to_string()))); // FIXME to_string is a hack
            },
            Some(Ok(b)) => b,
        };

        // cf. https://doc.rust-lang.org/src/core/char/methods.rs.html#884-886
        if b1 <= 0x7F { // ASCII
            Some(Ok(unsafe { std::char::from_u32_unchecked(u32::from(b1)) }))
        } else if b1 & 0b1110_0000 == 0b1100_0000 { // 2 bytes
            let b2 = match self.pop() {
                Err(e) => {
                    return Some(Err(e));
                }
                Ok(b) => b,
            };
            // https://rust-lang-nursery.github.io/rust-clippy/v0.0.212/index.html#cast_lossless
            let u: u32 = (u32::from(b1 & 0b0001_1111) << 6)
                       |  u32::from(b2 & 0b0011_1111);
            Some(Ok(unsafe { std::char::from_u32_unchecked(u) }))
        } else if b1 & 0b1111_0000 == 0b1110_0000 { // 3 bytes
            let b2 = match self.pop() {
                Err(e) => {
                    return Some(Err(e));
                }
                Ok(b) => b,
            };
            let b3 = match self.pop() {
                Err(e) => {
                    return Some(Err(e));
                }
                Ok(b) => b,
            };
            let u: u32 = (u32::from(b1 & 0b0000_1111) << 12)
                       | (u32::from(b2 & 0b0011_1111) << 6)
                       |  u32::from(b3 & 0b0011_1111);
            Some(Ok(unsafe { std::char::from_u32_unchecked(u) }))
        } else { // 4 bytes
            assert!(b1 & 0b1111_1000 == 0b1111_0000);
            let b2 = match self.pop() {
                Err(e) => {
                    return Some(Err(e));
                }
                Ok(b) => b,
            };
            let b3 = match self.pop() {
                Err(e) => {
                    return Some(Err(e));
                }
                Ok(b) => b,
            };
            let b4 = match self.pop() {
                Err(e) => {
                    return Some(Err(e));
                }
                Ok(b) => b,
            };
            // https://rust-lang-nursery.github.io/rust-clippy/v0.0.212/index.html#unreadable_literal
            let u: u32 = u32::from(b1 & 0b0000_0111)
                       | u32::from(b2 & 0b0011_1111)
                       | u32::from(b3 & 0b0011_1111)
                       | u32::from(b4 & 0b0011_1111);
            Some(Ok(unsafe { std::char::from_u32_unchecked(u) }))
        }
    }
}

pub fn decode_utf8<I>(iter: I) -> DecodeUtf8<I> {
    DecodeUtf8 {
        iter
    }
}

pub struct EncodeUtf8<I> {
    pub iter: I,
    pub buf: [u8; 4],
    pub index: usize,
}

impl<I, E> Iterator for EncodeUtf8<I>
    where I: Iterator<Item=Result<char, E>> {
    type Item = Result<u8, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 4 {
            let res = self.buf[self.index];
            self.index += 1;
            if self.index < 4 && self.buf[self.index] == 0 {
                self.index = 4;
            }
            Some(Ok(res))
        } else {
            match self.iter.next() {
                None => None,
                Some(Err(e)) => Some(Err(e)),
                Some(Ok(c)) => {
                    let len = c.encode_utf8(&mut self.buf).len();
                    if len > 1 {
                        self.index = 1;
                        if len < 4 {
                            self.buf[len] = 0;
                        }
                    }
                    Some(Ok(self.buf[0]))
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
    where I: Iterator<Item=Result<u8, E>>,
          W: Write,
          E: From<io::Error> {

    const SIZE: usize = 4096;
    let mut buf = [0; SIZE];
    let mut i = 0;

    for next in iter {
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
