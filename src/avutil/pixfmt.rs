use crate::ffi::avcodec_find_best_pix_fmt_of_list;
pub use crate::ffi::AVPixelFormat;

pub fn find_best_pix_fmt_of_list(
  possible_pix_fmts: Vec<AVPixelFormat>,
  src_pix_fmt: &AVPixelFormat,
  has_alpha: bool,
) -> (AVPixelFormat, libc::c_int) {
  let mut loss_ptr: libc::c_int = 0;
  let fmt = unsafe {
    avcodec_find_best_pix_fmt_of_list(
      possible_pix_fmts.as_ptr(),
      *src_pix_fmt,
      has_alpha as libc::c_int,
      &mut loss_ptr,
    )
  };
  (fmt, loss_ptr)
}
