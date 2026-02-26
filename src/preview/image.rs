use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

pub enum ImageLoadResult {
  Loaded(StatefulProtocol),
  Error(String),
}

fn is_jxl(path: &Path) -> bool {
  path.extension().is_some_and(|e| e.eq_ignore_ascii_case("jxl"))
}

fn load_jxl(path: &Path) -> Result<DynamicImage, String> {
  let file = File::open(path).map_err(|e| format!("Failed to open JXL file: {e}"))?;
  let reader = BufReader::new(file);
  let decoder =
    jxl_oxide::integration::JxlDecoder::new(reader).map_err(|e| format!("Failed to decode JXL: {e}"))?;
  DynamicImage::from_decoder(decoder).map_err(|e| format!("Failed to convert JXL to image: {e}"))
}

/// Load an image in a background thread, returning the result via channel
pub fn load_image_async(
  path: &Path,
  picker: &Picker,
) -> mpsc::Receiver<ImageLoadResult> {
  let (tx, rx) = mpsc::channel();
  let path = path.to_path_buf();
  let picker = picker.clone();

  thread::spawn(move || {
    let img_result = if is_jxl(&path) {
      load_jxl(&path)
    } else {
      image::open(&path).map_err(|e| format!("Failed to load image: {e}"))
    };
    let result = match img_result {
      Ok(img) => {
        let protocol = picker.new_resize_protocol(img);
        ImageLoadResult::Loaded(protocol)
      }
      Err(e) => ImageLoadResult::Error(e),
    };
    let _ = tx.send(result);
  });

  rx
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_load_nonexistent_image() {
    #[allow(deprecated)]
    let picker = Picker::from_fontsize((8, 16));
    let rx = load_image_async(Path::new("/nonexistent/image.png"), &picker);
    let result = rx.recv().unwrap();
    assert!(matches!(result, ImageLoadResult::Error(_)));
  }
}
