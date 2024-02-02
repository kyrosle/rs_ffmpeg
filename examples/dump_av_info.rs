use std::ffi::{CStr, CString};

use rs_ffmpeg::avformat::AVFormatContextInput;
use std::error::Error;

fn dump_av_info(path: &CStr) -> Result<(), Box<dyn Error>> {
  let mut dict = None;
  let mut input_format_context =
    AVFormatContextInput::open(path, None, &mut dict)?;
  input_format_context.dump(0, path)?;
  Ok(())
}

fn main() {
  dump_av_info(&CString::new("./examples/bear.mp4").unwrap()).unwrap();
}
