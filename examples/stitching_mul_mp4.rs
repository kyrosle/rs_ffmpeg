use anyhow::{anyhow, bail, Context, Result};
use cstr::cstr;
use rs_ffmpeg::{
  self,
  avcodec::{AVCodec, AVCodecContext},
  avformat::{
    AVFormatContextInput, AVFormatContextOutput, AVIOContextContainer,
    AVIOContextCustom,
  },
  avutil::{av_inv_q, av_mul_q, AVFrame, AVMem, AVRational},
  error::RsmpegError,
  ffi::{self, AV_CODEC_ID_H264},
  swscale::SwsContext,
};
use std::{
  ffi::CStr,
  fs::File,
  io::{Seek, SeekFrom, Write},
  sync::{Arc, Mutex},
};

/// Get `video_stream_index`, `input_format_context`, `decode_context`.
fn open_input_file(
  filename: &CStr,
) -> Result<(usize, AVFormatContextInput, AVCodecContext)> {
  let mut input_format_context =
    AVFormatContextInput::open(filename, None, &mut None)?;
  input_format_context.dump(0, filename)?;

  let (video_index, decoder) = input_format_context
    .find_best_stream(ffi::AVMEDIA_TYPE_VIDEO)
    .context("Failed to select a video stream")?
    .context("No video stream")?;

  let decode_context = {
    let input_stream = &input_format_context.streams()[video_index];

    let mut decode_context = AVCodecContext::new(&decoder);
    decode_context.apply_codecpar(&input_stream.codecpar())?;
    if let Some(framerate) = input_stream.guess_framerate() {
      decode_context.set_framerate(framerate);
    }
    decode_context.open(None)?;
    decode_context
  };

  Ok((video_index, input_format_context, decode_context))
}

/// Return output_format_context and encode_context
fn open_output_file(
  filename: &CStr,
  decode_context: &AVCodecContext,
  fix_width: Option<i32>,
  fix_height: Option<i32>,
) -> Result<(AVFormatContextOutput, AVCodecContext)> {
  let buffer = Arc::new(Mutex::new(File::create(filename.to_str()?)?));
  let buffer1 = buffer.clone();

  // Custom IO Context
  let io_context = AVIOContextCustom::alloc_context(
    AVMem::new(4096),
    true,
    vec![],
    None,
    Some(Box::new(move |_: &mut Vec<u8>, buf: &[u8]| {
      let mut buffer = buffer1.lock().unwrap();
      buffer.write_all(buf).unwrap();
      buf.len() as _
    })),
    Some(Box::new(
      move |_: &mut Vec<u8>, offset: i64, whence: i32| {
        println!("offset: {}, whence: {}", offset, whence);
        let mut buffer = match buffer.lock() {
          Ok(x) => x,
          Err(_) => return -1,
        };
        let mut seek_ = |offset: i64, whence: i32| -> Result<i64> {
          Ok(match whence {
            libc::SEEK_CUR => buffer.seek(SeekFrom::Current(offset))?,
            libc::SEEK_SET => buffer.seek(SeekFrom::Start(offset as u64))?,
            libc::SEEK_END => buffer.seek(SeekFrom::End(offset))?,
            _ => return Err(anyhow!("Unsupported whence")),
          } as i64)
        };
        seek_(offset, whence).unwrap_or(-1)
      },
    )),
  );

  let mut output_format_context = AVFormatContextOutput::create(
    filename,
    Some(AVIOContextContainer::Custom(io_context)),
  )?;

  let encoder = AVCodec::find_encoder(AV_CODEC_ID_H264)
    .with_context(|| anyhow!("encoder({}) not found.", AV_CODEC_ID_H264))?;

  let mut encode_context = AVCodecContext::new(&encoder);
  encode_context.set_height(if let Some(fix_height) = fix_height {
    fix_height
  } else {
    decode_context.height
  });
  encode_context.set_width(if let Some(fix_width) = fix_width {
    fix_width
  } else {
    decode_context.width
  });
  // encode_context.set_sample_aspect_ratio(
  //   if let (Some(height), Some(width)) = (fix_height, fix_width) {
  //     AVRational {
  //       num: width,
  //       den: height,
  //     }
  //   } else {
  //     decode_context.sample_aspect_ratio
  //   },
  // );
  encode_context.set_pix_fmt(if let Some(pix_fmts) = encoder.pix_fmts() {
    pix_fmts[0]
  } else {
    decode_context.pix_fmt
  });
  encode_context.set_time_base(av_inv_q(av_mul_q(
    decode_context.framerate,
    AVRational {
      num: decode_context.ticks_per_frame,
      den: 1,
    },
  )));

  // Some formats want stream headers to be separate.
  if output_format_context.oformat().flags & ffi::AVFMT_GLOBALHEADER as i32 != 0
  {
    encode_context.set_flags(
      encode_context.flags | ffi::AV_CODEC_FLAG_GLOBAL_HEADER as i32,
    );
  }

  encode_context.open(None)?;

  {
    let mut out_stream = output_format_context.new_stream();
    out_stream.set_codecpar(encode_context.extract_codecpar());
    out_stream.set_time_base(encode_context.time_base);
  }

  output_format_context.dump(0, filename)?;
  output_format_context.write_header(&mut None)?;

  Ok((output_format_context, encode_context))
}

/// encode -> write_frame
fn encode_write_frame(
  frame_after: Option<&AVFrame>,
  encode_context: &mut AVCodecContext,
  output_format_context: &mut AVFormatContextOutput,
  out_stream_index: usize,
) -> Result<()> {
  encode_context
    .send_frame(frame_after)
    .context("Encode frame failed.")?;

  loop {
    let mut packet = match encode_context.receive_packet() {
      Ok(packet) => packet,
      Err(RsmpegError::EncoderDrainError)
      | Err(RsmpegError::EncoderFlushedError) => break,
      Err(e) => bail!(e),
    };

    packet.set_stream_index(out_stream_index as i32);
    packet.rescale_ts(
      encode_context.time_base,
      output_format_context.streams()[out_stream_index].time_base,
    );

    match output_format_context.interleaved_write_frame(&mut packet) {
      Ok(()) => Ok(()),
      Err(RsmpegError::InterleavedWriteFrameError(-22)) => Ok(()),
      Err(e) => Err(e),
    }
    .context("Interleaved write frame failed.")?;
  }

  Ok(())
}

/// Send an empty packet to the `encode_context` for packet flushing.
fn flush_encoder(
  encode_context: &mut AVCodecContext,
  output_format_context: &mut AVFormatContextOutput,
  out_stream_index: usize,
) -> Result<()> {
  if encode_context.codec().capabilities & ffi::AV_CODEC_CAP_DELAY as i32 == 0 {
    return Ok(());
  }
  encode_write_frame(
    None,
    encode_context,
    output_format_context,
    out_stream_index,
  )?;
  Ok(())
}

/// Transcoding audio and video stream in a multi media file.
pub fn stitching_mul_mp4(
  input_file_1: &CStr,
  input_file_2: &CStr,
  output_file: &CStr,
) -> Result<()> {
  let (video_stream_index_1, mut input_format_context_1, mut decode_context_1) =
    open_input_file(input_file_1)?;
  let (video_stream_index_2, mut input_format_context_2, mut decode_context_2) =
    open_input_file(input_file_2)?;

  let max_width: i32 = decode_context_1.width.max(decode_context_2.width);
  let max_height: i32 = decode_context_1.height.max(decode_context_2.height);

  let (mut output_format_context, mut encode_context) = open_output_file(
    output_file,
    &decode_context_1,
    Some(max_width),
    Some(max_height),
  )?;

  let mut last_pts = 0;

  loop {
    let mut packet = match input_format_context_1.read_packet() {
      Ok(Some(x)) => x,
      // No more frames
      Ok(None) => break,
      Err(e) => bail!("Read frame error: {:?}", e),
    };

    if packet.stream_index as usize != video_stream_index_1 {
      continue;
    }

    packet.rescale_ts(
      input_format_context_1.streams()[video_stream_index_1].time_base,
      encode_context.time_base,
    );

    decode_context_1
      .send_packet(Some(&packet))
      .context("Send packet failed")?;

    loop {
      let frame = match decode_context_1.receive_frame() {
        Ok(frame) => frame,
        Err(RsmpegError::DecoderDrainError)
        | Err(RsmpegError::DecoderFlushedError) => break,
        Err(e) => bail!(e),
      };

      let mut new_frame = AVFrame::create(
        ffi::AV_PIX_FMT_YUV420P,
        encode_context.width,
        encode_context.height,
        1,
      )
      .context("Failed to create image buffer.")?;

      let mut sws_context = SwsContext::get_context(
        decode_context_1.width,
        decode_context_1.height,
        decode_context_1.pix_fmt,
        decode_context_1.width,
        decode_context_1.height,
        encode_context.pix_fmt,
        ffi::SWS_FAST_BILINEAR,
        None,
        None,
        None,
      )
      .context("Failed to create a swscale context.")?;
      sws_context.scale_frame(
        &frame,
        0,
        decode_context_1.height,
        &mut new_frame,
      )?;
      last_pts = frame.best_effort_timestamp;
      new_frame.set_pts(frame.best_effort_timestamp);
      new_frame.set_pkt_dts(frame.best_effort_timestamp);

      encode_write_frame(
        Some(&new_frame),
        &mut encode_context,
        &mut output_format_context,
        0,
      )?;
    }
  }

  last_pts += 1;

  loop {
    let mut packet = match input_format_context_2.read_packet() {
      Ok(Some(x)) => x,
      // No more frames
      Ok(None) => break,
      Err(e) => bail!("Read frame error: {:?}", e),
    };

    if packet.stream_index as usize != video_stream_index_2 {
      continue;
    }

    packet.rescale_ts(
      input_format_context_2.streams()[video_stream_index_2].time_base,
      encode_context.time_base,
    );

    decode_context_2
      .send_packet(Some(&packet))
      .context("Send packet failed")?;

    loop {
      let frame = match decode_context_2.receive_frame() {
        Ok(frame) => frame,
        Err(RsmpegError::DecoderDrainError)
        | Err(RsmpegError::DecoderFlushedError) => break,
        Err(e) => bail!(e),
      };

      let mut new_frame = AVFrame::create(
        ffi::AV_PIX_FMT_YUV420P,
        encode_context.width,
        encode_context.height,
        1,
      )
      .context("Failed to create image buffer.")?;

      let mut sws_context = SwsContext::get_context(
        decode_context_2.width,
        decode_context_2.height,
        decode_context_2.pix_fmt,
        encode_context.width,
        encode_context.height,
        encode_context.pix_fmt,
        ffi::SWS_FAST_BILINEAR,
        None,
        None,
        None,
      )
      .context("Failed to create a swscale context.")?;
      sws_context.scale_frame(
        &frame,
        0,
        decode_context_2.height,
        &mut new_frame,
      )?;
      new_frame.set_pts(last_pts + frame.best_effort_timestamp);
      new_frame.set_pkt_dts(last_pts + frame.pkt_dts);
      encode_write_frame(
        Some(&new_frame),
        &mut encode_context,
        &mut output_format_context,
        0,
      )?;
    }
  }

  // Flush the encoder by pushing EOF frame to encode_context.
  flush_encoder(&mut encode_context, &mut output_format_context, 0)?;
  output_format_context.write_trailer()?;
  Ok(())
}

fn main() {
  std::fs::create_dir_all("examples/output/stitching_mul_mp4/").unwrap();
  stitching_mul_mp4(
    cstr!("examples/assets/bear.mp4"),
    cstr!("examples/assets/test.mp4"),
    cstr!("examples/output/stitching_mul_mp4/stitching_mp4.mp4"),
  )
  .unwrap();
}
