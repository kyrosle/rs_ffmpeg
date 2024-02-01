use std::ffi::CString;

use rs_ffmpeg::{
  avcodec::{AVCodec, AVCodecContext},
  avformat::{AVFormatContextInput, AVFormatContextOutput},
  error::Result,
  ffi::{
    av_packet_unref, AVCodecID, AVFMT_GLOBALHEADER, AV_CODEC_ID_H264,
    AV_PIX_FMT_YUV420P,
  },
};

const OUTPUT_CODEC: AVCodecID = AV_CODEC_ID_H264;

fn main() -> Result<()> {
  let file_name_1 = CString::new("./examples/bear.mp4").unwrap();
  let file_name_2 = CString::new("./examples/test.mp4").unwrap();
  let output_file_name = CString::new("./example/concat_mul.mp4").unwrap();

  let mut options: Option<_> = None;
  let mut format_ctx1 =
    AVFormatContextInput::open(&file_name_1, None, &mut options)?;
  let mut format_ctx2 =
    AVFormatContextInput::open(&file_name_2, None, &mut options)?;

  let mut output_format_ctx =
    AVFormatContextOutput::create(&output_file_name, None)?;

  let output_format = output_format_ctx.oformat();
  {
    let mut out_stream = output_format_ctx.new_stream();

    let codec = AVCodec::find_decoder(AV_CODEC_ID_H264).unwrap();
    let mut codec_ctx = AVCodecContext::new(&codec);
    codec_ctx.set_codec_id(OUTPUT_CODEC);
    codec_ctx.set_bit_rate(400000);
    codec_ctx.set_width(format_ctx1.streams()[0].codecpar().width);
    codec_ctx.set_height(format_ctx1.streams()[0].codecpar().height);
    codec_ctx.set_time_base(format_ctx1.streams()[0].time_base);
    codec_ctx
      .set_framerate(format_ctx1.streams()[0].guess_framerate().unwrap());
    codec_ctx.set_gop_size(12);
    codec_ctx.set_pix_fmt(AV_PIX_FMT_YUV420P);

    if output_format.flags & (AVFMT_GLOBALHEADER as i32) == 1 {
      codec_ctx.set_flags(codec_ctx.flags | (AVFMT_GLOBALHEADER as i32));
    }

    out_stream.set_codecpar(codec_ctx.extract_codecpar());
    codec_ctx.open(None)?;
  }
  {
    let mut dict: Option<_> = None;
    output_format_ctx.write_header(&mut dict)?;
  }

  while let Some(mut pkt) = format_ctx1.read_packet()? {
    let out_stream = &output_format_ctx.streams()[0];
    pkt.set_stream_index(out_stream.index);
    output_format_ctx.interleaved_write_frame(&mut pkt)?;
    unsafe { av_packet_unref(pkt.as_mut_ptr()) };
  }
  while let Some(mut pkt) = format_ctx2.read_packet()? {
    let out_stream = &output_format_ctx.streams()[0];
    pkt.set_stream_index(out_stream.index);
    output_format_ctx.interleaved_write_frame(&mut pkt)?;
    unsafe { av_packet_unref(pkt.as_mut_ptr()) };
  }

  output_format_ctx.write_trailer()?;

  Ok(())
}
