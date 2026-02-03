use std::path::Path;
use std::sync::mpsc;
use std::thread;

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

pub enum ImageLoadResult {
  Loaded(StatefulProtocol),
  Error(String),
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
    let result = match image::open(&path) {
      Ok(img) => {
        let protocol = picker.new_resize_protocol(img);
        ImageLoadResult::Loaded(protocol)
      }
      Err(e) => ImageLoadResult::Error(format!("Failed to load image: {e}")),
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
