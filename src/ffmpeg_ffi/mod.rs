mod avutil;

#[allow(
  non_snake_case,
  non_camel_case_types,
  non_upper_case_globals,
  improper_ctypes,
  clippy::all
)]
pub mod ffi {
  pub use super::avutil::{
    _avutil::*, common::*, error::*, pixfmt::*, rational::*,
  };
  include!("binding.rs");
}
