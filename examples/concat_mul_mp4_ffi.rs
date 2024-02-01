// doing
use std::{ffi::CString, ptr};

use rs_ffmpeg::ffi::{
  self, av_write_trailer, avcodec_open2, avcodec_parameters_copy, avformat_alloc_output_context2, avformat_close_input, avformat_find_stream_info, avformat_free_context, avformat_new_stream, avformat_open_input, avformat_write_header, avio_open
};
fn main() {
  let file_name_1 = CString::new("./examples/bear.mp4").unwrap();
  let file_name_2 = CString::new("./examples/test.mp4").unwrap();
  let output_file_name = CString::new("./example/concat_mul.mp4").unwrap();

  let mut input_format_context_ptr = unsafe { ffi::avformat_alloc_context() };
  if input_format_context_ptr.is_null() {
    panic!("ERROR could not allocate memory for Format Context");
  }

  let mut output_format_context_ptr = unsafe { ffi::avformat_alloc_context() };
  if output_format_context_ptr.is_null() {
    panic!("ERROR could not allocate memory for Format Context");
  }

  let output_format_context =
    unsafe { output_format_context_ptr.as_mut() }.unwrap();

  let mut ret: i32 = 0;

  let avcodec_param_copy = move |file_name: CString| {
    if unsafe {
      ret = avformat_open_input(
        &mut input_format_context_ptr,
        file_name.as_ptr(),
        ptr::null_mut(),
        ptr::null_mut(),
      );
      ret
    } != 0
    {
      println!("ret: {}, file_name: {}", ret, file_name.to_str().unwrap());
      panic!("ERROR could not open the file bear.mp4");
    }

    let input_format_context =
      unsafe { input_format_context_ptr.as_mut() }.unwrap();

    if unsafe {
      ret = avformat_find_stream_info(input_format_context, ptr::null_mut());
      ret
    } != 0
    {
      println!("ret: {}, file_name: {}", ret, file_name.to_str().unwrap());
      panic!("Error finding stream information");
    }

    let streams = unsafe {
      std::slice::from_raw_parts(
        input_format_context.streams,
        input_format_context.nb_streams as usize,
      )
    };

    for stream in streams {
      let output_stream =
        unsafe { avformat_new_stream(output_format_context, ptr::null_mut()) };
      if output_stream.is_null() {
        panic!("Error creating output stream");
      }

      let output_stream = unsafe { *output_stream };
      let output_stream_codecpar = output_stream.codecpar;
      let output_stream_codec = output_stream.priv_data;
      let stream = unsafe { **stream };
      let stream_codecpar = stream.codecpar;

      if unsafe {
        ret = avcodec_parameters_copy(output_stream_codecpar, stream_codecpar);
        ret
      } != 0
      {
        panic!("Error opening codec");
      }

      // if unsafe {
      //   ret = avcodec_open2(output_stream., codec, options)
      // }
    }

    unsafe { avformat_close_input(&mut input_format_context_ptr) };
  };

  // Create output file
  if unsafe {
    avformat_alloc_output_context2(
      &mut output_format_context_ptr,
      ptr::null_mut(),
      ptr::null_mut(),
      output_file_name.as_ptr(),
    )
  } != 0
  {
    panic!("Error creating output file")
  }

  let output_format_context =
    unsafe { output_format_context_ptr.as_mut() }.unwrap();

  // Write file header
  if unsafe {
    ret = avformat_write_header(output_format_context, ptr::null_mut());
    ret
  } != 0
  {
    println!("ret: {}", ret);
    panic!("Error writing header");
  }

  // Read packets from input files and write them to output file

  // Write file trailer
  if unsafe {
    ret = av_write_trailer(output_format_context);
    ret
  } != 0
  {
    println!("ret: {}", ret);
    panic!("Error writing trailer");
  }

  // Clean up
  // unsafe { avformat_close_input(&mut input_format_context_ptr_1) };
  // unsafe { avformat_close_input(&mut input_format_context_ptr_2) };
  unsafe { avformat_free_context(output_format_context_ptr) };
}
