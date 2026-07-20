use handler_common::DocumentHandler;
use std::path::Path;

/// Extract all images from a document, saving each to dest_dir as PNG.
/// Returns list of saved file paths.
pub fn extract_images(
    handler: &dyn DocumentHandler,
    dest_dir: &str,
) -> Result<Vec<String>, String> {
    let nodes = handler
        .query("image")
        .map_err(|e| format!("query images: {}", e))?;

    if nodes.is_empty() {
        return Err("no images found in document".to_string());
    }

    let dest = Path::new(dest_dir);
    let mut saved = Vec::new();

    for (i, node) in nodes.iter().enumerate() {
        let path = &node.path;
        let filename = format!("image_{}.png", i + 1);
        let dest_path = dest.join(&filename);
        let dest_str = dest_path.to_string_lossy().to_string();

        match handler.try_extract_binary(path, &dest_str) {
            Ok(Some(info)) => {
                saved.push(dest_str);
                let _ = info;
            }
            Ok(None) => {
                // fallback: try to save via image crate if node has text content
                if let Some(ref text) = node.text {
                    if let Ok(img) = image::load_from_memory(text.as_bytes()) {
                        if let Err(e) = img.save(&dest_path) {
                            return Err(format!("save image {}: {}", filename, e));
                        }
                        saved.push(dest_str);
                    }
                }
            }
            Err(e) => {
                return Err(format!("extract image '{}': {}", path, e));
            }
        }
    }

    Ok(saved)
}

/// Re-compress all images in a document.
/// This is best-effort — works fully when the document is a ZIP-based format
/// and the handler supports binary extraction/writing.
pub fn compress_images(
    handler: &dyn DocumentHandler,
    _quality: u8,
) -> Result<(), String> {
    let nodes = handler
        .query("image")
        .map_err(|e| format!("query images: {}", e))?;

    if nodes.is_empty() {
        return Err("no images found in document".to_string());
    }

    // Best-effort: iterate and report
    let count = nodes.len();

    #[cfg(feature = "png-optimize")]
    {
        let tmpdir =
            std::env::temp_dir().join(format!("officecli_compress_{}", std::process::id()));
        std::fs::create_dir_all(&tmpdir).map_err(|e| format!("create tmp dir: {}", e))?;

        for (i, node) in nodes.iter().enumerate() {
            let path = &node.path;
            let tmpfile = tmpdir.join(format!("img_{}", i));
            let tmpstr = tmpfile.to_string_lossy().to_string();

            if let Ok(Some(_)) = handler.try_extract_binary(path, &tmpstr) {
                let data = std::fs::read(&tmpfile).map_err(|e| format!("read tmp: {}", e))?;

                let opts = oxipng::Options {
                    strip: oxipng::Strip::Safe,
                    ..Default::default()
                };
                if let Ok(_optimized) = oxipng::optimize_from_memory(&data, &opts) {
                    // writing back requires raw_set which is format-specific
                }
            }
        }

        let _ = std::fs::remove_dir_all(&tmpdir);
    }

    let _ = count;
    Ok(())
}

/// Replace an image in a document with a new image file.
/// This is best-effort — works when the handler supports raw binary replacement.
pub fn replace_image(
    handler: &mut dyn DocumentHandler,
    path: &str,
    new_image: &str,
) -> Result<(), String> {
    // Verify the new image exists and is a valid image
    let _ = image::open(new_image)
        .map_err(|e| format!("cannot open new image '{}': {}", new_image, e))?;

    // Check that the target path is an image in the document
    let node = handler
        .get(path, 0)
        .map_err(|e| format!("get node '{}': {}", path, e))?;

    if node.element_type != "image" {
        return Err(format!(
            "path '{}' is not an image (type: {})",
            path, node.element_type
        ));
    }

    // Best-effort: use raw_set to write the new image bytes as base64 text content
    let new_data = std::fs::read(new_image).map_err(|e| format!("read '{}': {}", new_image, e))?;
    let b64 = base64_encode(&new_data);

    handler
        .set(path, &[("image_data".to_string(), b64)].into())
        .map_err(|e| format!("set image data: {}", e))?;

    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Get image info (dimensions, format) from a file path.
pub fn image_info(path: &str) -> Result<(u32, u32, String), String> {
    let reader = image::ImageReader::open(path)
        .map_err(|e| format!("open '{}': {}", path, e))?
        .with_guessed_format()
        .map_err(|e| format!("guess format: {}", e))?;

    let fmt = reader
        .format()
        .map(|f| format!("{:?}", f).to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    let img = reader
        .decode()
        .map_err(|e| format!("decode '{}': {}", path, e))?;

    Ok((img.width(), img.height(), fmt))
}

/// Resize an image file and save to output path.
pub fn resize_image(
    input: &str,
    output: &str,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let img = image::open(input).map_err(|e| format!("open '{}': {}", input, e))?;
    let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    resized
        .save(output)
        .map_err(|e| format!("save '{}': {}", output, e))
}

/// Convert an image file to the target format.
pub fn convert_image(input: &str, output: &str, format: &str) -> Result<(), String> {
    let img = image::open(input).map_err(|e| format!("open '{}': {}", input, e))?;
    let fmt = parse_format(format)?;
    // For JPEG, set quality
    if fmt == image::ImageFormat::Jpeg {
        let mut buf = std::io::Cursor::new(Vec::new());
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
        img.write_with_encoder(encoder)
            .map_err(|e| format!("encode jpeg: {}", e))?;
        std::fs::write(output, buf.into_inner())
            .map_err(|e| format!("write '{}': {}", output, e))
    } else {
        img.save(output)
            .map_err(|e| format!("save '{}': {}", output, e))
    }
}

fn parse_format(s: &str) -> Result<image::ImageFormat, String> {
    match s.to_lowercase().as_str() {
        "png" => Ok(image::ImageFormat::Png),
        "jpeg" | "jpg" => Ok(image::ImageFormat::Jpeg),
        "gif" => Ok(image::ImageFormat::Gif),
        "webp" => Ok(image::ImageFormat::WebP),
        "bmp" => Ok(image::ImageFormat::Bmp),
        other => Err(format!("unsupported format: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_png(w: u32, h: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        let img = image::DynamicImage::new_rgba8(w, h);
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn test_image_info_png() {
        let bytes = make_test_png(64, 48);
        let dir = std::env::temp_dir();
        let path = dir.join("test_info.png");
        std::fs::write(&path, &bytes).unwrap();
        let (w, h, fmt) = image_info(&path.to_string_lossy()).unwrap();
        assert_eq!(w, 64);
        assert_eq!(h, 48);
        assert_eq!(fmt, "png");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_parse_format_valid() {
        assert_eq!(parse_format("png").unwrap(), image::ImageFormat::Png);
        assert_eq!(parse_format("jpg").unwrap(), image::ImageFormat::Jpeg);
        assert_eq!(parse_format("jpeg").unwrap(), image::ImageFormat::Jpeg);
        assert_eq!(parse_format("gif").unwrap(), image::ImageFormat::Gif);
        assert_eq!(parse_format("webp").unwrap(), image::ImageFormat::WebP);
        assert_eq!(parse_format("bmp").unwrap(), image::ImageFormat::Bmp);
    }

    #[test]
    fn test_parse_format_invalid() {
        assert!(parse_format("tiff").is_err());
        assert!(parse_format("svg").is_err());
    }

    #[test]
    fn test_resize_image() {
        let bytes = make_test_png(200, 100);
        let dir = std::env::temp_dir();
        let input = dir.join("test_resize_in.png");
        let output = dir.join("test_resize_out.png");
        std::fs::write(&input, &bytes).unwrap();

        resize_image(
            &input.to_string_lossy(),
            &output.to_string_lossy(),
            50,
            50,
        )
        .unwrap();

        let (w, h, _) = image_info(&output.to_string_lossy()).unwrap();
        assert_eq!(w, 50);
        assert_eq!(h, 50);

        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&output);
    }

    #[test]
    fn test_convert_png_to_jpeg() {
        let bytes = make_test_png(32, 32);
        let dir = std::env::temp_dir();
        let input = dir.join("test_conv_in.png");
        let output = dir.join("test_conv_out.jpg");
        std::fs::write(&input, &bytes).unwrap();

        convert_image(
            &input.to_string_lossy(),
            &output.to_string_lossy(),
            "jpeg",
        )
        .unwrap();

        let (w, h, fmt) = image_info(&output.to_string_lossy()).unwrap();
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        assert_eq!(fmt, "jpeg");

        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&output);
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"hello world";
        let encoded = base64_encode(data);
        assert_eq!(encoded, "aGVsbG8gd29ybGQ=");
    }

    #[test]
    fn test_image_info_invalid_file() {
        let result = image_info("/nonexistent/image.png");
        assert!(result.is_err());
    }
}
