use std::io::{Writer, IoError, IoResult};
use std::num::Int;

use rustc_serialize::Encoder;

pub struct EncoderWriter<'a, W: 'a> {
    writer: &'a mut W,
}

impl <'a, W: Writer> EncoderWriter<'a, W> {
    pub fn new(w: &'a mut W) -> EncoderWriter<'a, W> {
        EncoderWriter {
            writer: w,
        }
    }
}

impl<'a, W: Writer> Encoder for EncoderWriter<'a, W> {
    type Error = IoError;

    fn emit_nil(&mut self) -> IoResult<()> { Ok(()) }
    fn emit_usize(&mut self, v: usize) -> IoResult<()> {
        self.emit_u64(v as u64)
    }
    fn emit_u64(&mut self, v: u64) -> IoResult<()> {
        self.writer.write_be_u64(v)
    }
    fn emit_u32(&mut self, v: u32) -> IoResult<()> {
        self.writer.write_be_u32(v)
    }
    fn emit_u16(&mut self, v: u16) -> IoResult<()> {
        self.writer.write_be_u16(v)
    }
    fn emit_u8(&mut self, v: u8) -> IoResult<()> {
        self.writer.write_u8(v)
    }
    fn emit_isize(&mut self, v: isize) -> IoResult<()> {
        self.emit_i64(v as i64)
    }
    fn emit_i64(&mut self, v: i64) -> IoResult<()> {
        self.writer.write_be_i64(v)
    }
    fn emit_i32(&mut self, v: i32) -> IoResult<()> {
        self.writer.write_be_i32(v)
    }
    fn emit_i16(&mut self, v: i16) -> IoResult<()> {
        self.writer.write_be_i16(v)
    }
    fn emit_i8(&mut self, v: i8) -> IoResult<()> {
        self.writer.write_i8(v)
    }
    fn emit_bool(&mut self, v: bool) -> IoResult<()> {
        self.writer.write_u8(if v {1} else {0})
    }
    fn emit_f64(&mut self, v: f64) -> IoResult<()> {
        self.writer.write_be_f64(v)
    }
    fn emit_f32(&mut self, v: f32) -> IoResult<()> {
        self.writer.write_be_f32(v)
    }
    fn emit_char(&mut self, v: char) -> IoResult<()> {
        self.writer.write_char(v)
    }
    fn emit_str(&mut self, v: &str) -> IoResult<()> {
        try!(self.emit_usize(v.len()));
        self.writer.write_str(v)
    }
    fn emit_enum<F>(&mut self, __: &str, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_enum_variant<F>(&mut self, _: &str,
                            v_id: usize,
                            _: usize,
                            f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            let max: u8 = Int::max_value();
            if v_id > (max as usize) {
                panic!("Variant tag doesn't fit in a u8")
            }
            try!(self.emit_u8(v_id as u8));
            f(self)
        }
    fn emit_enum_variant_arg<F>(&mut self, _: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_enum_struct_variant<F>(&mut self, _: &str,
                                   _: usize,
                                   _: usize,
                                   f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_enum_struct_variant_field<F>(&mut self,
                                         _: &str,
                                         _: usize,
                                         f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_struct<F>(&mut self, _: &str, _: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_struct_field<F>(&mut self, _: &str, _: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_tuple<F>(&mut self, _: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_tuple_arg<F>(&mut self, _: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_tuple_struct<F>(&mut self, _: &str, len: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            self.emit_tuple(len, f)
        }
    fn emit_tuple_struct_arg<F>(&mut self, f_idx: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            self.emit_tuple_arg(f_idx, f)
        }
    fn emit_option<F>(&mut self, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_option_none(&mut self) -> IoResult<()> {
        self.writer.write_u8(0)
    }
    fn emit_option_some<F>(&mut self, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            try!(self.writer.write_u8(1));
            f(self)
        }
    fn emit_seq<F>(&mut self, len: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            try!(self.emit_usize(len));
            f(self)
        }
    fn emit_seq_elt<F>(&mut self, _: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_map<F>(&mut self, len: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            try!(self.emit_usize(len));
            f(self)
        }
    fn emit_map_elt_key<F>(&mut self, _: usize, mut f: F) -> IoResult<()> where
        F: FnMut(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
    fn emit_map_elt_val<F>(&mut self, _: usize, f: F) -> IoResult<()> where
        F: FnOnce(&mut EncoderWriter<'a, W>) -> IoResult<()> {
            f(self)
        }
}
