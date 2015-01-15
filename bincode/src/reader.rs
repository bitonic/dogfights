use std::io::{Buffer, Reader, IoError};
use std::error::{Error, FromError};
use std::fmt;

use rustc_serialize::Decoder;

#[derive(PartialEq, Clone, Show)]
pub struct InvalidBytes {
    desc: &'static str,
    detail: Option<String>,
}

impl fmt::String for InvalidBytes {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InvalidBytes { detail: None, desc } =>
                write!(fmt, "{}", desc),
            InvalidBytes { detail: Some(ref detail), desc } =>
                write!(fmt, "{} ({})", desc, detail)
        }
    }
}

#[derive(PartialEq, Clone, Show)]
pub enum DecodingError {
    IoError(IoError),
    InvalidBytes(InvalidBytes),
}

impl fmt::String for DecodingError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecodingError::IoError(ref ioerr) =>
                write!(fmt, "IoError: {}", ioerr),
            DecodingError::InvalidBytes(ref ib) =>
                write!(fmt, "InvalidBytes: {}", ib)
        }
    }
}

pub type DecodingResult<T> = Result<T, DecodingError>;

fn wrap_io(err: IoError) -> DecodingError {
    DecodingError::IoError(err)
}

impl Error for DecodingError {
    fn description(&self) -> &str {
        match *self {
            DecodingError::IoError(ref err)     => err.description(),
            DecodingError::InvalidBytes(ref ib) => ib.desc,
        }
    }

    fn detail(&self) -> Option<String> {
        match *self {
            DecodingError::IoError(ref err)     => err.detail(),
            DecodingError::InvalidBytes(ref ib) => ib.detail.clone(),
        }
    }
}

impl FromError<IoError> for DecodingError {
    fn from_error(err: IoError) -> DecodingError {
        DecodingError::IoError(err)
    }
}

pub struct DecoderReader<'a, R: 'a> {
    reader: &'a mut R,
}

impl<'a, R: Reader+Buffer> DecoderReader<'a, R> {
    pub fn new(r: &'a mut R) -> DecoderReader<'a, R> {
        DecoderReader {
            reader: r,
        }
    }
}

impl<'a, R: Reader+Buffer> Decoder for DecoderReader<'a, R> {
    type Error = DecodingError;

    fn read_nil(&mut self) -> DecodingResult<()> {
        Ok(())
    }
    fn read_usize(&mut self) -> DecodingResult<usize> {
        Ok(try!(self.read_u64().map(|x| x as usize)))
    }
    fn read_u64(&mut self) -> DecodingResult<u64> {
        self.reader.read_be_u64().map_err(wrap_io)
    }
    fn read_u32(&mut self) -> DecodingResult<u32> {
        self.reader.read_be_u32().map_err(wrap_io)
    }
    fn read_u16(&mut self) -> DecodingResult<u16> {
        self.reader.read_be_u16().map_err(wrap_io)
    }
    fn read_u8(&mut self) -> DecodingResult<u8> {
        self.reader.read_u8().map_err(wrap_io)
    }
    fn read_isize(&mut self) -> DecodingResult<isize> {
        self.read_i64().map(|x| x as isize)
    }
    fn read_i64(&mut self) -> DecodingResult<i64> {
        self.reader.read_be_i64().map_err(wrap_io)
    }
    fn read_i32(&mut self) -> DecodingResult<i32> {
        self.reader.read_be_i32().map_err(wrap_io)
    }
    fn read_i16(&mut self) -> DecodingResult<i16> {
        self.reader.read_be_i16().map_err(wrap_io)
    }
    fn read_i8(&mut self) -> DecodingResult<i8> {
        self.reader.read_i8().map_err(wrap_io)
    }
    fn read_bool(&mut self) -> DecodingResult<bool> {
        let x = try!(self.read_i8());
        match x {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(DecodingError::InvalidBytes(InvalidBytes{
                desc: "invalid u8 when decoding bool",
                detail: Some(format!("Expected 0 or 1, got {}", x))
            })),
        }
    }
    fn read_f64(&mut self) -> DecodingResult<f64> {
        self.reader.read_be_f64().map_err(wrap_io)
    }
    fn read_f32(&mut self) -> DecodingResult<f32> {
        self.reader.read_be_f32().map_err(wrap_io)
    }
    fn read_char(&mut self) -> DecodingResult<char> {
        self.reader.read_char().map_err(wrap_io)
    }
    fn read_str(&mut self) -> DecodingResult<String> {
        let len = try!(self.read_usize());

        let vector = try!(self.reader.read_exact(len));
        match String::from_utf8(vector) {
            Ok(s) => Ok(s),
            Err(err) => Err(DecodingError::InvalidBytes(InvalidBytes {
                desc: "error while decoding utf8 string",
                detail: Some(format!("Decoding error: {}", err))
            })),
        }
    }
    fn read_enum<T, F>(&mut self, _: &str, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            f(self)
        }
    fn read_enum_variant<T, F>(&mut self, names: &[&str], mut f: F) -> DecodingResult<T> where
        F: FnMut(&mut DecoderReader<'a, R>, usize) -> DecodingResult<T> {
            let id = try!(self.read_u8());
            let id = id as usize;
            if id >= names.len() {
                Err(DecodingError::InvalidBytes(InvalidBytes {
                    desc: "out of bounds tag when reading enum variant",
                    detail: Some(format!("Expected tag < {}, got {}", names.len(), id))
                }))
            } else {
                f(self, id)
            }
        }
    fn read_enum_variant_arg<T, F>(&mut self, _: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            f(self)
        }
    fn read_enum_struct_variant<T, F>(&mut self, names: &[&str], f: F) -> DecodingResult<T> where
        F: FnMut(&mut DecoderReader<'a, R>, usize) -> DecodingResult<T> {
            self.read_enum_variant(names, f)
        }
    fn read_enum_struct_variant_field<T, F>(&mut self,
                                            _: &str,
                                            f_idx: usize,
                                            f: F)
        -> DecodingResult<T> where
            F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
                self.read_enum_variant_arg(f_idx, f)
            }
    fn read_struct<T, F>(&mut self, _: &str, _: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            f(self)
        }
    fn read_struct_field<T, F>(&mut self,
                               _: &str,
                               _: usize,
                               f: F)
        -> DecodingResult<T> where
            F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
                f(self)
            }
    fn read_tuple<T, F>(&mut self, _: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            f(self)
        }
    fn read_tuple_arg<T, F>(&mut self, _: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            f(self)
        }
    fn read_tuple_struct<T, F>(&mut self, _: &str, len: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            self.read_tuple(len, f)
        }
    fn read_tuple_struct_arg<T, F>(&mut self, a_idx: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            self.read_tuple_arg(a_idx, f)
        }
    fn read_option<T, F>(&mut self, mut f: F) -> DecodingResult<T> where
        F: FnMut(&mut DecoderReader<'a, R>, bool) -> DecodingResult<T> {
            let x = try!(self.read_u8());
            match x {
                1 => f(self, true),
                0 => f(self, false),
                _ => Err(DecodingError::InvalidBytes(InvalidBytes {
                    desc: "invalid tag when decoding Option",
                    detail: Some(format!("Expected 0 or 1, got {}", x))
                })),
            }
        }
    fn read_seq<T, F>(&mut self, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>, usize) -> DecodingResult<T> {
            let len = try!(self.read_usize());
            f(self, len)
        }
    fn read_seq_elt<T, F>(&mut self, _: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            f(self)
        }
    fn read_map<T, F>(&mut self, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>, usize) -> DecodingResult<T> {
            let len = try!(self.read_usize());
            f(self, len)
        }
    fn read_map_elt_key<T, F>(&mut self, _: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            f(self)
        }
    fn read_map_elt_val<T, F>(&mut self, _: usize, f: F) -> DecodingResult<T> where
        F: FnOnce(&mut DecoderReader<'a, R>) -> DecodingResult<T> {
            f(self)
        }
    fn error(&mut self, err: &str) -> DecodingError {
        DecodingError::InvalidBytes(InvalidBytes {
            desc: "user-induced error",
            detail: Some(err.to_string()),
        })
    }
}
