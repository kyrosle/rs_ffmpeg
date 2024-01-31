#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::module_inception)]
#![allow(clippy::upper_case_acronyms)]

mod ffmpeg_ffi;

pub use ffmpeg_ffi::ffi;
