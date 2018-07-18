use std::io::{self, Read, Write};
use std::error::Error;
use std::collections::HashMap;

struct DecodeUtf8<I> {
    iter: I,
}
impl<I, E> Iterator for DecodeUtf8<I>
    where I: Iterator<Item=Result<u8, E>>,
          E: Error {
    type Item = Result<char, Box<Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf: [u8; 4] = [0; 4];
        let mut i = 0;

        loop {
            match self.iter.next() {
                None => {
                    if i == 0 {
                        return None;
                    } else {
                        return Some(Err(From::from("Invalid UTF-8 sequence"))); // FIXME more info format!("Invalid UTF-8 sequence: {:?}", buf[..i]))));
                    }
                },
                Some(Err(e)) => {
                    return Some(Err(From::from(e.to_string()))); // FIXME to_string is a hack
                },
                Some(Ok(b)) => {
                    buf[i] = b;
                    i += 1;
                    match std::str::from_utf8(&buf[..i]) {
                        Err(_) => (),
                        Ok(str) => {
                            match str.chars().next() {
                                None => {
                                    panic!("This does not make sense");
                                },
                                Some(c) => {
                                    return Some(Ok(c));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn decode_utf8<I>(iter: I) -> DecodeUtf8<I> {
    DecodeUtf8 {
        iter
    }
}

struct EncodeUtf8<I> {
    iter: I,
    buf: [u8; 4],
    index: usize,
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

fn encode_utf8<I>(iter: I) -> EncodeUtf8<I> {
    EncodeUtf8 {
        iter,
        buf: [0; 4],
        index: 4,
    }
}

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
    let text_output = text_input.map(|res| res.map(|c| mapper.get(&c).unwrap_or(&c).clone()));
    //let text_output = "שלום עשח".chars().map(|c| (Ok(c) as Result<char, Box<Error>>)); // FIXME remove
    let byte_output = encode_utf8(text_output);

    connect(byte_output, stdout.lock())?;

    Ok(())
}

fn connect<I, W, E>(iter: I, mut hout: W) -> Result<(), E>
    where I: Iterator<Item=Result<u8, E>>,
          W: Write,
          E: From<io::Error> {

    const SIZE: usize = 4096;
    let mut buf = [0; SIZE];
    let mut i = 0;

    for next in iter {
        let b = next?;
        buf[i] = b;
        i = i + 1;

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
