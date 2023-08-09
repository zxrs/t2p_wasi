use anyhow::{ensure, Result};
use std::ffi::CString;
use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::slice;

mod sys;
use sys::{
    t2p_err_t_T2P_ERR_ERROR as T2P_ERR_ERROR, t2p_free, t2p_init, t2p_write_pdf, tdata_t,
    thandle_t, TIFFClientOpen, TIFFClose, T2P, TIFF,
};

fn main() -> Result<()> {
    let tiff = fs::read("/input")?;
    let pdf = generate_pdf(&tiff)?;
    fs::write("/output", &pdf)?;
    Ok(())
}
struct T2p<'a>(&'a T2P);

impl<'a> Drop for T2p<'a> {
    fn drop(&mut self) {
        unsafe { t2p_free(self.as_mut_ptr()) };
    }
}

impl<'a> T2p<'a> {
    fn as_mut_ptr(&self) -> *mut T2P {
        self.0 as *const _ as _
    }
}

struct Tiff<'a>(&'a TIFF);

impl<'a> Drop for Tiff<'a> {
    fn drop(&mut self) {
        unsafe { TIFFClose(self.as_mut_ptr()) };
    }
}

impl<'a> Tiff<'a> {
    fn as_mut_ptr(&self) -> *mut TIFF {
        self.0 as *const _ as _
    }
}

struct Input<'a>(Cursor<&'a [u8]>);

struct Output<'a>(Cursor<&'a mut Vec<u8>>);

unsafe extern "C" fn input_read(handle: thandle_t, buf: tdata_t, size: i64) -> i64 {
    let input_memory = &mut *(handle as *mut Input);
    let buf = slice::from_raw_parts_mut(buf as *mut u8, size as _);
    input_memory.0.read_exact(buf).expect("failed to read.");
    size
}

unsafe extern "C" fn input_write(_: thandle_t, _: tdata_t, _: i64) -> i64 {
    0
}

unsafe extern "C" fn input_seek(handle: thandle_t, offset: u64, whence: i32) -> u64 {
    let input_memory = &mut *(handle as *mut Input);
    let pos = match whence {
        0 => SeekFrom::Start(offset),
        1 => SeekFrom::Current(offset as _),
        2 => SeekFrom::End(offset as _),
        _ => unimplemented!(),
    };
    input_memory.0.seek(pos).expect("failed to seek.")
}

unsafe extern "C" fn output_read(_: thandle_t, _: tdata_t, _: i64) -> i64 {
    0
}

unsafe extern "C" fn output_write(handle: thandle_t, data: tdata_t, size: i64) -> i64 {
    let output_memory = &mut *(handle as *mut Output);
    //dbg!(omem.0.position());
    let data = slice::from_raw_parts(data as *const u8, size as _);
    output_memory
        .0
        .write_all(data)
        .expect("failed to write data");
    size
}

unsafe extern "C" fn output_seek(_: thandle_t, offset: u64, _: i32) -> u64 {
    offset
}

unsafe extern "C" fn dummy_close(_: thandle_t) -> i32 {
    0
}

unsafe extern "C" fn dummy_size(_: thandle_t) -> u64 {
    0
}

unsafe extern "C" fn dummy_map(_: thandle_t, _: *mut tdata_t, _: *mut u64) -> i32 {
    0
}

unsafe extern "C" fn dummy_unmap(_: thandle_t, _: tdata_t, _: u64) {}

fn generate_pdf(tiff: &[u8]) -> Result<Vec<u8>> {
    let t2p = unsafe { t2p_init() };
    ensure!(!t2p.is_null(), "failed to t2p init.");
    let t2p = T2p(unsafe { &*t2p });

    let input_memory = Input(Cursor::new(tiff));
    let mut buf = vec![];
    let mut output_memory = Output(Cursor::new(&mut buf));

    let name = CString::new("MemoryInput")?;
    let mode = CString::new("rm")?;
    let input_tiff = unsafe {
        TIFFClientOpen(
            name.as_ptr(),
            mode.as_ptr(),
            &input_memory as *const Input as _,
            Some(input_read),
            Some(input_write),
            Some(input_seek),
            Some(dummy_close),
            Some(dummy_size),
            Some(dummy_map),
            Some(dummy_unmap),
        )
    };
    ensure!(!input_tiff.is_null(), "failed to open input tiff.");
    let input_tiff = Tiff(unsafe { &*input_tiff });

    let name = CString::new("MemoryOutput")?;
    let mode = CString::new("wb")?;
    let output_tiff = unsafe {
        TIFFClientOpen(
            name.as_ptr(),
            mode.as_ptr(),
            &output_memory as *const Output as _,
            Some(output_read),
            Some(output_write),
            Some(output_seek),
            Some(dummy_close),
            Some(dummy_size),
            Some(dummy_map),
            Some(dummy_unmap),
        )
    };
    ensure!(!output_tiff.is_null(), "failed to open output tiff.");
    let output_tiff = Tiff(unsafe { &*output_tiff });

    output_memory.0.set_position(0);

    let ret = unsafe {
        t2p_write_pdf(
            t2p.as_mut_ptr(),
            input_tiff.as_mut_ptr(),
            output_tiff.as_mut_ptr(),
        )
    };
    ensure!(
        ret != 0 && t2p.0.t2p_error != T2P_ERR_ERROR,
        "failed to write pdf."
    );

    Ok(buf)
}
