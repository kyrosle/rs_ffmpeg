#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::module_inception)]
#![allow(clippy::upper_case_acronyms)]

mod ffmpeg_ffi;

pub use ffmpeg_ffi::ffi;

#[macro_use]
mod macros;

mod shared;

pub mod avcodec;
pub mod avfilter;
pub mod avformat;
pub mod avutil;
pub mod swresample;
pub mod swscale;

pub mod error;

pub use shared::UnsafeDerefMut;
