//! todo: support linux/window, use vcpkg instead of git
use std::{
  collections::HashSet,
  env, fs,
  io::Result,
  path::{Path, PathBuf},
  process::Command,
};

use bindgen::{callbacks, Bindings};
use once_cell::sync::Lazy;
// ------------------------ static variables ------------------------
static TEMP_PATH: Lazy<String> = Lazy::new(|| {
  format!(
    "{}/target/tmp",
    env::var("CARGO_MANIFEST_DIR").expect("Failed to get out_dir directory"),
  )
});
static FFMPEG_PATH: Lazy<String> = Lazy::new(|| {
  format!(
    "{}/target/tmp/ffmpeg",
    env::var("CARGO_MANIFEST_DIR").expect("Failed to get out_dir directory"),
  )
});

static FFMPEG_BUILD_PATH: Lazy<String> = Lazy::new(|| {
  format!(
    "{}/target/tmp/ffmpeg_build",
    env::var("CARGO_MANIFEST_DIR").expect("Failed to get out_dir directory"),
  )
});

// ------------------------ git_clone_ffmpeg_source_and_build_lib ------------------------

fn pwd() -> Result<PathBuf> {
  std::env::current_dir()
}

fn cd(dir_name: &str) -> Result<()> {
  std::env::set_current_dir(dir_name)
}

fn debug_print_pwd() {
  println!("current path: {}", pwd().unwrap().display());
}

fn git_clone_ffmpeg_source_and_build_lib() -> Result<()> {
  println!("ffmpeg_path = {}", *FFMPEG_PATH);
  println!("ffmpeg_build_path = {}", *FFMPEG_BUILD_PATH);
  let current_path = pwd()?;
  fs::create_dir_all(TEMP_PATH.clone())?;
  println!("pwd: {:#?}", current_path);

  if fs::metadata(FFMPEG_PATH.clone()).is_err()
    || fs::metadata(FFMPEG_BUILD_PATH.clone()).is_err()
  {
    // cd(TEMP_PATH)?;
    let build_path = FFMPEG_BUILD_PATH.clone();
    let branch = std::env::args()
      .nth(1)
      .unwrap_or_else(|| "release/6.1".to_string());

    if fs::metadata(FFMPEG_PATH.clone()).is_err() {
      Command::new("git")
        .arg("clone")
        .arg("--single-branch")
        .arg("--branch")
        .arg(&branch)
        .arg("--depth")
        .arg("1")
        .arg("https://github.com/ffmpeg/ffmpeg")
        .arg(FFMPEG_PATH.clone())
        .status()?;
    }

    cd(&FFMPEG_PATH.clone())?;
    debug_print_pwd();
    Command::new("git")
      .arg("fetch")
      .arg("origin")
      .arg(&branch)
      .arg("--depth")
      .arg("1")
      .status()?;

    Command::new("git")
      .arg("checkout")
      .arg("FETCH_HEAD")
      .status()?;

    Command::new("./configure")
      .arg(format!("--prefix={}", build_path))
      .arg("--enable-gpl")
      // .arg("--enable-libass")
      // .arg("--enable-libfdk-aac")
      // .arg("--enable-libfreetype")
      // .arg("--enable-libmp3lame")
      // .arg("--enable-libopus")
      // .arg("--enable-libvorbis")
      .arg("--enable-libvpx")
      .arg("--enable-libx264")
      // .arg("--enable-libx265")
      // To workaround `https://github.com/larksuite/rs_ffmpeg/pull/98#issuecomment-1467511193`
      .arg("--disable-decoder=exr,phm")
      .arg("--enable-protocol=file")
      .arg("--disable-sdl2")
      .arg("--disable-programs")
      .arg("--enable-nonfree")
      .status()?;

    Command::new("make")
      .arg("-j")
      .arg(num_cpus::get().to_string())
      .status()?;

    Command::new("make").arg("install").status()?;

    let _ = cd(current_path.to_str().unwrap_or("."));
    println!(
      "finished clone ffmpeg source and build lib current path: {}",
      pwd()?.display()
    );
  }

  Ok(())
}

// ------------------------ create_ffmpeg_rust_bindgen ------------------------

/// All the libs that FFmpeg has
static LIBS: Lazy<[&str; 7]> = Lazy::new(|| {
  [
    "avcodec",
    "avdevice",
    "avfilter",
    "avformat",
    "avutil",
    "swresample",
    "swscale",
  ]
});

/// Whitelist of the headers we want to generate bindings
static HEADERS: Lazy<Vec<PathBuf>> = Lazy::new(|| {
  [
    "libavcodec/ac3_parser.h",
    "libavcodec/adts_parser.h",
    "libavcodec/avcodec.h",
    "libavcodec/avdct.h",
    "libavcodec/avfft.h",
    "libavcodec/bsf.h",
    "libavcodec/codec.h",
    "libavcodec/codec_desc.h",
    "libavcodec/codec_id.h",
    "libavcodec/codec_par.h",
    // "libavcodec/d3d11va.h",
    "libavcodec/defs.h",
    "libavcodec/dirac.h",
    "libavcodec/dv_profile.h",
    // "libavcodec/dxva2.h",
    "libavcodec/jni.h",
    "libavcodec/mediacodec.h",
    "libavcodec/packet.h",
    // "libavcodec/qsv.h",
    // "libavcodec/vdpau.h",
    "libavcodec/version.h",
    "libavcodec/version_major.h",
    // "libavcodec/videotoolbox.h",
    "libavcodec/vorbis_parser.h",
    // "libavcodec/xvmc.h",
    "libavdevice/avdevice.h",
    "libavdevice/version.h",
    "libavdevice/version_major.h",
    "libavfilter/avfilter.h",
    "libavfilter/buffersink.h",
    "libavfilter/buffersrc.h",
    "libavfilter/version.h",
    "libavfilter/version_major.h",
    "libavformat/avformat.h",
    "libavformat/avio.h",
    "libavformat/version.h",
    "libavformat/version_major.h",
    "libavutil/adler32.h",
    "libavutil/aes.h",
    "libavutil/aes_ctr.h",
    "libavutil/ambient_viewing_environment.h",
    "libavutil/attributes.h",
    "libavutil/audio_fifo.h",
    "libavutil/avassert.h",
    "libavutil/avconfig.h",
    "libavutil/avstring.h",
    "libavutil/avutil.h",
    "libavutil/base64.h",
    "libavutil/blowfish.h",
    "libavutil/bprint.h",
    "libavutil/bswap.h",
    "libavutil/buffer.h",
    "libavutil/camellia.h",
    "libavutil/cast5.h",
    "libavutil/channel_layout.h",
    "libavutil/common.h",
    "libavutil/cpu.h",
    "libavutil/crc.h",
    "libavutil/csp.h",
    "libavutil/des.h",
    "libavutil/detection_bbox.h",
    "libavutil/dict.h",
    "libavutil/display.h",
    "libavutil/dovi_meta.h",
    "libavutil/downmix_info.h",
    "libavutil/encryption_info.h",
    "libavutil/error.h",
    "libavutil/eval.h",
    "libavutil/executor.h",
    "libavutil/ffversion.h",
    "libavutil/fifo.h",
    "libavutil/file.h",
    "libavutil/film_grain_params.h",
    "libavutil/frame.h",
    "libavutil/hash.h",
    "libavutil/hdr_dynamic_metadata.h",
    "libavutil/hdr_dynamic_vivid_metadata.h",
    "libavutil/hmac.h",
    "libavutil/hwcontext.h",
    // "libavutil/hwcontext_cuda.h",
    // "libavutil/hwcontext_d3d11va.h",
    // "libavutil/hwcontext_drm.h",
    // "libavutil/hwcontext_dxva2.h",
    // "libavutil/hwcontext_mediacodec.h",
    // "libavutil/hwcontext_opencl.h",
    // "libavutil/hwcontext_qsv.h",
    // "libavutil/hwcontext_vaapi.h",
    // "libavutil/hwcontext_vdpau.h",
    // "libavutil/hwcontext_videotoolbox.h",
    // "libavutil/hwcontext_vulkan.h",
    "libavutil/imgutils.h",
    "libavutil/intfloat.h",
    "libavutil/intreadwrite.h",
    "libavutil/lfg.h",
    "libavutil/log.h",
    "libavutil/lzo.h",
    "libavutil/macros.h",
    "libavutil/mastering_display_metadata.h",
    "libavutil/mathematics.h",
    "libavutil/md5.h",
    "libavutil/mem.h",
    "libavutil/motion_vector.h",
    "libavutil/murmur3.h",
    "libavutil/opt.h",
    "libavutil/parseutils.h",
    "libavutil/pixdesc.h",
    "libavutil/pixelutils.h",
    "libavutil/pixfmt.h",
    "libavutil/random_seed.h",
    "libavutil/rational.h",
    "libavutil/rc4.h",
    "libavutil/replaygain.h",
    "libavutil/ripemd.h",
    "libavutil/samplefmt.h",
    "libavutil/sha.h",
    "libavutil/sha512.h",
    "libavutil/spherical.h",
    "libavutil/stereo3d.h",
    "libavutil/tea.h",
    "libavutil/threadmessage.h",
    "libavutil/time.h",
    "libavutil/timecode.h",
    "libavutil/timestamp.h",
    "libavutil/tree.h",
    "libavutil/twofish.h",
    "libavutil/tx.h",
    "libavutil/uuid.h",
    "libavutil/version.h",
    "libavutil/video_enc_params.h",
    "libavutil/video_hint.h",
    "libavutil/xtea.h",
    "libswresample/swresample.h",
    "libswresample/version.h",
    "libswresample/version_major.h",
    "libswscale/swscale.h",
    "libswscale/version.h",
    "libswscale/version_major.h",
  ]
  .into_iter()
  .map(|x| Path::new(x).iter().collect())
  .collect()
});

#[derive(Debug)]
struct FilterCargoCallbacks {
  emitted_macro: HashSet<&'static str>,
}

impl FilterCargoCallbacks {
  fn new(set: HashSet<&'static str>) -> Self {
    FilterCargoCallbacks { emitted_macro: set }
  }
}

impl callbacks::ParseCallbacks for FilterCargoCallbacks {
  fn will_parse_macro(&self, name: &str) -> callbacks::MacroParsingBehavior {
    if self.emitted_macro.contains(name) {
      callbacks::MacroParsingBehavior::Ignore
    } else {
      callbacks::MacroParsingBehavior::Default
    }
  }
}

fn use_prebuilt_binding(from: &Path, to: &Path) {
  fs::copy(from, to).unwrap_or_else(|_| {
    panic!(
      "Prebult binding file failed to be copied! from: {}, to: {}",
      from.display(),
      to.display()
    )
  });
}

fn copy_output_binding(from: &Path, to: &Path) {
  fs::copy(from, to).unwrap_or_else(|_| {
    panic!(
      "Output binding file failed to be copied! from: {}, to: {}",
      from.display(),
      to.display()
    )
  });
}

fn generate_bindings(
  ffmpeg_include_dir: &Path,
  headers: &[PathBuf],
) -> Bindings {
  if !Path::new(ffmpeg_include_dir).exists() {
    panic!(
      "FFmpeg include dir: `{:?}` doesn't exits",
      ffmpeg_include_dir
    );
  }
  // Because of the strange `FP_*` in `math.h` https://github.com/rust-lang/rust-bindgen/issues/687
  let filter_callback = FilterCargoCallbacks::new(
    [
      "FP_NAN",
      "FP_INFINITE",
      "FP_ZERO",
      "FP_SUBNORMAL",
      "FP_NORMAL",
    ]
    .into_iter()
    .collect(),
  );
  // Bindgen on all avaiable headers
  headers
    .iter()
    .map(|header| ffmpeg_include_dir.join(header))
    .filter(|path| path.exists())
    // .filter(|path| {
    //   let exists = Path::new(&path).exists();
    //   if !exists {
    //     eprintln!("Header path `{:?}` not found.", path);
    //   }
    //   exists
    // })
    .fold(
      {
        bindgen::builder()
          // Force impl Debug if possible(for `AVCodecParameters`)
          .impl_debug(true)
          .parse_callbacks(Box::new(filter_callback))
          // Add clang path, for `#include` header finding in bindgen process.
          .clang_arg(format!("-I{}", ffmpeg_include_dir.display()))
          // Stop bindgen from prefixing enums
          .prepend_enum_name(false)
      },
      |builder, header| builder.header(header.to_string_lossy()),
    )
    .generate()
    .expect("Binding generation failed.")
}

#[allow(dead_code)]
pub struct EnvVars {
  docs_rs: Option<String>,
  out_dir: Option<PathBuf>,
  ffmpeg_include_dir: Option<PathBuf>,
  ffmpeg_dll_path: Option<PathBuf>,
  ffmpeg_pkg_config_path: Option<PathBuf>,
  ffmpeg_libs_dir: Option<PathBuf>,
  ffmpeg_binding_path: Option<PathBuf>,
}

impl EnvVars {
  fn init() -> Self {
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=OUT_DIR");
    println!("cargo:rerun-if-env-changed=FFMPEG_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=FFMPEG_DLL_PATH");
    println!("cargo:rerun-if-env-changed=FFMPEG_PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=FFMPEG_LIBS_DIR");
    println!("cargo:rerun-if-env-changed=FFMPEG_BINDING_PATH");

    Self {
      docs_rs: env::var("DOCS_RS").ok(),
      out_dir: env::var("OUT_DIR").ok().map(remove_verbatim),
      ffmpeg_include_dir: env::var("FFMPEG_INCLUDE_DIR")
        .ok()
        .map(remove_verbatim),
      ffmpeg_dll_path: env::var("FFMPEG_DLL_PATH").ok().map(remove_verbatim),
      ffmpeg_pkg_config_path: env::var("FFMPEG_PKG_CONFIG_PATH")
        .or::<String>(Ok(format!(
          "{}/lib/pkgconfig",
          FFMPEG_BUILD_PATH.clone()
        )))
        .ok()
        .map(remove_verbatim),

      ffmpeg_libs_dir: env::var("FFMPEG_LIBS_DIR").ok().map(remove_verbatim),
      ffmpeg_binding_path: env::var("FFMPEG_BINDING_PATH")
        .ok()
        .map(remove_verbatim),
    }
  }
}

/// clang doesn't support -I{verbatim path} on windows, so we need to remove it if possible.
fn remove_verbatim(path: String) -> PathBuf {
  let path = if let Some(path) = path.strip_prefix(r#"\\?\"#) {
    path.to_string()
  } else {
    path
  };
  PathBuf::from(path)
}

fn static_linking_with_libs_dir(
  library_names: &[&str],
  ffmpeg_libs_dir: &Path,
) {
  println!(
    "cargo:rustc-link-search=native={}",
    ffmpeg_libs_dir.display()
  );
  for library_name in library_names {
    println!("cargo:rustc-link-lib=static={}", library_name);
  }
}

fn static_linking(env_vars: &EnvVars) {
  let output_binding_path =
    &env_vars.out_dir.as_ref().unwrap().join("binding.rs");

  println!("output_binding_path: {}", output_binding_path.display());

  fn link_and_bindgen(env_vars: &EnvVars, output_binding_path: &Path) {
    // Probe libraries(enable emitting cargo metadata)
    let include_paths = static_linking_with_pkg_config(&*LIBS);
    if let Some(ffmpeg_binding_path) = env_vars.ffmpeg_binding_path.as_ref() {
      use_prebuilt_binding(ffmpeg_binding_path, output_binding_path);
    } else if let Some(ffmpeg_include_dir) =
      env_vars.ffmpeg_include_dir.as_ref()
    {
      // If use ffmpeg_pkg_config_path with ffmpeg_include_dir, prefer using the user given dir rather than pkg_config_path.
      generate_bindings(ffmpeg_include_dir, &HEADERS)
        .write_to_file(output_binding_path)
        .expect("Cannot write binding to file.");
    } else {
      generate_bindings(&include_paths[0], &HEADERS)
        .write_to_file(output_binding_path)
        .expect("Cannot write binding to file.");
    }

    let manifest_dir =
      env::var("CARGO_MANIFEST_DIR").expect("Failed to get manifest directory");
    let binding_code_path = Path::new(&manifest_dir)
      .join("src")
      .join("ffmpeg_ffi")
      .join("binding.rs");
    copy_output_binding(output_binding_path, &binding_code_path);
  }
  use non_windows::*;
  // Hint: set PKG_CONFIG_PATH to some placeholder value will let pkg_config probing system library.
  if let Some(ffmpeg_pkg_config_path) = env_vars.ffmpeg_pkg_config_path.as_ref()
  {
    if !Path::new(ffmpeg_pkg_config_path).exists() {
      panic!(
        "error: FFMPEG_PKG_CONFIG_PATH is set to `{}`, which does not exist.",
        ffmpeg_pkg_config_path.display()
      );
    }
    env::set_var("PKG_CONFIG_PATH", ffmpeg_pkg_config_path);
    link_and_bindgen(env_vars, output_binding_path);
  } else if let Some(ffmpeg_libs_dir) = env_vars.ffmpeg_libs_dir.as_ref() {
    static_linking_with_libs_dir(&*LIBS, ffmpeg_libs_dir);
    if let Some(ffmpeg_binding_path) = env_vars.ffmpeg_binding_path.as_ref() {
      use_prebuilt_binding(ffmpeg_binding_path, output_binding_path);
    } else if let Some(ffmpeg_include_dir) =
      env_vars.ffmpeg_include_dir.as_ref()
    {
      generate_bindings(ffmpeg_include_dir, &HEADERS)
        .write_to_file(output_binding_path)
        .expect("Cannot write binding to file.");
    } else {
      panic!("No binding generation method is set!");
    }
  } else {
    #[cfg(feature = "link_system_ffmpeg")]
    link_and_bindgen(env_vars, output_binding_path);
    #[cfg(not(feature = "link_system_ffmpeg"))]
    panic!("No linking method set!");
  };
}

fn docs_rs_linking(env_vars: &EnvVars) {
  // If it's a documentation generation from docs.rs, just copy the bindings
  // generated locally to `OUT_DIR`. We do this because the building
  // environment of docs.rs doesn't have an network connection, so we cannot
  // git clone the FFmpeg. And they also have a limitation on crate's size:
  // 10MB, which is not enough to fit in FFmpeg source files. So the only
  // thing we can do is copying the locally generated binding files to the
  // `OUT_DIR`.
  let binding_file_path =
    &env_vars.out_dir.as_ref().unwrap().join("binding.rs");
  use_prebuilt_binding(Path::new("src/binding.rs"), binding_file_path);
}

fn create_ffmpeg_rust_bindgen(env_vars: &EnvVars) {
  if env_vars.docs_rs.is_some() {
    docs_rs_linking(env_vars);
  } else {
    static_linking(env_vars);
  }
}

fn main() -> Result<()> {
  git_clone_ffmpeg_source_and_build_lib()?;
  let env_vars = EnvVars::init();
  create_ffmpeg_rust_bindgen(&env_vars);
  Ok(())
}

mod non_windows {
  use super::*;

  pub fn static_linking_with_pkg_config(
    library_names: &[&str],
  ) -> Vec<PathBuf> {
    // TODO: if specific library is not enabled, we should not probe it. If we
    // want to implement this, we Should modify try_probe_system_ffmpeg() too.
    let mut paths = HashSet::new();
    for libname in library_names {
      let new_paths = pkg_config::Config::new()
        // currently only support building with static libraries.
        .statik(true)
        .cargo_metadata(true)
        .probe(&format!("lib{}", libname))
        .unwrap_or_else(|e| panic!("{} not found! \nerror: {:#?}", libname, e))
        .include_paths;
      for new_path in new_paths {
        let new_path = new_path.to_str().unwrap().to_string();
        paths.insert(new_path);
      }
    }
    paths.into_iter().map(PathBuf::from).collect()
  }
}
