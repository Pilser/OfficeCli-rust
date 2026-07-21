use image::{DynamicImage, ImageEncoder};
use std::io::Cursor;

/// Image processing operations specification
#[derive(Debug, Clone)]
pub struct ImageOp {
    pub resize: Option<(u32, u32)>,
    pub crop: Option<(u32, u32, u32, u32)>,
    pub format: Option<String>,
    pub quality: Option<u8>,
    pub watermark: Option<WatermarkSpec>,
    pub rotate: Option<i32>,
    pub flip_h: bool,
    pub flip_v: bool,
    pub greyscale: bool,
}

impl Default for ImageOp {
    fn default() -> Self {
        Self {
            resize: None,
            crop: None,
            format: None,
            quality: None,
            watermark: None,
            rotate: None,
            flip_h: false,
            flip_v: false,
            greyscale: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WatermarkSpec {
    pub text: String,
    pub opacity: f32,
    pub position: String,
}

fn parse_image_format(s: &str) -> Result<image::ImageFormat, String> {
    match s.to_lowercase().as_str() {
        "png" => Ok(image::ImageFormat::Png),
        "jpg" | "jpeg" => Ok(image::ImageFormat::Jpeg),
        "gif" => Ok(image::ImageFormat::Gif),
        "webp" => Ok(image::ImageFormat::WebP),
        "bmp" => Ok(image::ImageFormat::Bmp),
        other => Err(format!("unsupported image format: {}", other)),
    }
}

fn load_image(bytes: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(bytes).map_err(|e| format!("failed to decode image: {}", e))
}

fn encode_image(
    img: &DynamicImage,
    fmt: image::ImageFormat,
    quality: Option<u8>,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);

    if fmt == image::ImageFormat::Jpeg {
        let q = quality.unwrap_or(85).min(100).max(1);
        let rgb = img.to_rgb8();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, q);
        encoder
            .write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ColorType::Rgb8.into())
            .map_err(|e| format!("failed to encode jpeg: {}", e))?;
    } else {
        img.write_to(&mut cursor, fmt)
            .map_err(|e| format!("failed to encode image: {}", e))?;
    }

    Ok(buf)
}

/// Process an image from raw bytes, returning processed bytes and new format hint.
pub fn process_image(
    input_bytes: &[u8],
    input_format: &str,
    ops: &ImageOp,
) -> Result<(Vec<u8>, String), String> {
    let input_fmt = parse_image_format(input_format)?;
    let target_fmt = match &ops.format {
        Some(f) => parse_image_format(f)?,
        None => input_fmt,
    };

    let img = load_image(input_bytes)?;

    let img = if let Some((x, y, w, h)) = ops.crop {
        img.crop_imm(x, y, w, h)
    } else {
        img
    };

    let img = if let Some((w, h)) = ops.resize {
        img.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let img = if let Some(deg) = ops.rotate {
        match deg {
            90 | -270 => img.rotate90(),
            180 | -180 => img.rotate180(),
            270 | -90 => img.rotate270(),
            _ => {
                return Err(format!(
                    "unsupported rotation: {}° (only 90, 180, 270 supported)",
                    deg
                ))
            }
        }
    } else {
        img
    };

    let mut img = img;
    if ops.flip_h {
        img = img.fliph();
    }
    if ops.flip_v {
        img = img.flipv();
    }
    if ops.greyscale {
        img = img.grayscale();
    }

    if let Some(ws) = &ops.watermark {
        apply_watermark(&mut img, ws);
    }

    let out_fmt_name = format_target_name(target_fmt, &ops.format);
    let bytes = encode_image(&img, target_fmt, ops.quality)?;
    Ok((bytes, out_fmt_name))
}

fn format_target_name(fmt: image::ImageFormat, user_specified: &Option<String>) -> String {
    user_specified
        .clone()
        .unwrap_or_else(|| format!("{:?}", fmt).to_lowercase())
}

fn apply_watermark(img: &mut DynamicImage, ws: &WatermarkSpec) {
    use image::GenericImage;
    use image::GenericImageView;
    use image::Pixel;

    let (w, h) = img.dimensions();
    let alpha = (ws.opacity.clamp(0.0, 1.0) * 255.0) as u8;

    let (rect_w, rect_h) = ((w / 4).max(1), (h / 8).max(1));
    let (x, y) = match ws.position.to_lowercase().as_str() {
        "topleft" => (0, 0),
        "topright" => (w.saturating_sub(rect_w), 0),
        "bottomleft" => (0, h.saturating_sub(rect_h)),
        "bottomright" => (w.saturating_sub(rect_w), h.saturating_sub(rect_h)),
        _ => ((w - rect_w) / 2, (h - rect_h) / 2),
    };

    for py in y..(y + rect_h).min(h) {
        for px in x..(x + rect_w).min(w) {
            let mut pixel = img.get_pixel(px, py);
            let channels = pixel.channels_mut();
            for c in 0..3.min(channels.len()) {
                channels[c] = (channels[c] as u16 * alpha as u16 / 255) as u8;
            }
            if channels.len() > 3 {
                channels[3] = (channels[3] as u16 * alpha as u16 / 255) as u8;
            }
            img.put_pixel(px, py, pixel);
        }
    }
}

/// Get image dimensions from raw bytes
pub fn image_dimensions(input_bytes: &[u8]) -> Result<(u32, u32), String> {
    use image::GenericImageView;
    let img = load_image(input_bytes)?;
    Ok(img.dimensions())
}

/// Convert image to target format bytes
pub fn convert_image(
    input_bytes: &[u8],
    target_format: &str,
    quality: Option<u8>,
) -> Result<Vec<u8>, String> {
    let fmt = parse_image_format(target_format)?;
    let img = load_image(input_bytes)?;
    encode_image(&img, fmt, quality)
}

/// Resize image maintaining aspect ratio (fit within max_w x max_h)
pub fn resize_image(input_bytes: &[u8], max_w: u32, max_h: u32) -> Result<Vec<u8>, String> {
    let img = load_image(input_bytes)?;
    let resized = img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
    encode_image(&resized, image::ImageFormat::Png, None)
}

/// Crop a region from the image
pub fn crop_image(
    input_bytes: &[u8],
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<Vec<u8>, String> {
    let img = load_image(input_bytes)?;
    let cropped = img.crop_imm(x, y, w, h);
    encode_image(&cropped, image::ImageFormat::Png, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    fn make_test_png(w: u32, h: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        let img = DynamicImage::new_rgba8(w, h);
        img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    fn make_test_jpeg(w: u32, h: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        let img = DynamicImage::new_rgba8(w, h).to_rgb8();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
        encoder.write_image(img.as_raw(), w, h, image::ColorType::Rgb8.into()).unwrap();
        buf
    }

    #[test]
    fn test_image_dimensions_png() {
        let bytes = make_test_png(64, 48);
        let (w, h) = image_dimensions(&bytes).unwrap();
        assert_eq!(w, 64);
        assert_eq!(h, 48);
    }

    #[test]
    fn test_image_dimensions_jpeg() {
        let bytes = make_test_jpeg(100, 200);
        let (w, h) = image_dimensions(&bytes).unwrap();
        assert_eq!(w, 100);
        assert_eq!(h, 200);
    }

    #[test]
    fn test_resize_smaller() {
        let bytes = make_test_png(200, 100);
        let resized = resize_image(&bytes, 50, 50).unwrap();
        let (w, h) = image_dimensions(&resized).unwrap();
        assert!(w <= 50);
        assert!(h <= 50);
    }

    #[test]
    fn test_resize_larger() {
        let bytes = make_test_png(10, 20);
        let resized = resize_image(&bytes, 100, 100).unwrap();
        let (w, h) = image_dimensions(&resized).unwrap();
        assert!(w <= 100);
        assert!(h <= 100);
    }

    #[test]
    fn test_convert_png_to_jpeg() {
        let bytes = make_test_png(32, 32);
        let converted = convert_image(&bytes, "jpeg", Some(90)).unwrap();
        let (w, h) = image_dimensions(&converted).unwrap();
        assert_eq!(w, 32);
        assert_eq!(h, 32);
    }

    #[test]
    fn test_convert_png_to_webp() {
        let bytes = make_test_png(16, 16);
        let converted = convert_image(&bytes, "webp", None).unwrap();
        let (w, h) = image_dimensions(&converted).unwrap();
        assert_eq!(w, 16);
        assert_eq!(h, 16);
    }

    #[test]
    fn test_crop() {
        let bytes = make_test_png(100, 100);
        let cropped = crop_image(&bytes, 10, 20, 30, 40).unwrap();
        let (w, h) = image_dimensions(&cropped).unwrap();
        assert_eq!(w, 30);
        assert_eq!(h, 40);
    }

    #[test]
    fn test_rotate_90() {
        let bytes = make_test_png(50, 100);
        let ops = ImageOp {
            rotate: Some(90),
            ..Default::default()
        };
        let (result, _) = process_image(&bytes, "png", &ops).unwrap();
        let (w, h) = image_dimensions(&result).unwrap();
        assert_eq!(w, 100);
        assert_eq!(h, 50);
    }

    #[test]
    fn test_rotate_180() {
        let bytes = make_test_png(50, 100);
        let ops = ImageOp {
            rotate: Some(180),
            ..Default::default()
        };
        let (result, _) = process_image(&bytes, "png", &ops).unwrap();
        let (w, h) = image_dimensions(&result).unwrap();
        assert_eq!(w, 50);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_rotate_270() {
        let bytes = make_test_png(50, 100);
        let ops = ImageOp {
            rotate: Some(270),
            ..Default::default()
        };
        let (result, _) = process_image(&bytes, "png", &ops).unwrap();
        let (w, h) = image_dimensions(&result).unwrap();
        assert_eq!(w, 100);
        assert_eq!(h, 50);
    }

    #[test]
    fn test_greyscale() {
        let bytes = make_test_png(10, 10);
        let ops = ImageOp {
            greyscale: true,
            ..Default::default()
        };
        let (result, _) = process_image(&bytes, "png", &ops).unwrap();
        let img = load_image(&result).unwrap();
        let pixel = img.get_pixel(0, 0);
        let channels = pixel.0;
        assert_eq!(channels[0], channels[1]);
        assert_eq!(channels[1], channels[2]);
    }

    #[test]
    fn test_flip_horizontal() {
        let bytes = make_test_png(20, 20);
        let ops = ImageOp {
            flip_h: true,
            ..Default::default()
        };
        let (result, _) = process_image(&bytes, "png", &ops).unwrap();
        let (w, h) = image_dimensions(&result).unwrap();
        assert_eq!(w, 20);
        assert_eq!(h, 20);
    }

    #[test]
    fn test_flip_vertical() {
        let bytes = make_test_png(20, 20);
        let ops = ImageOp {
            flip_v: true,
            ..Default::default()
        };
        let (result, _) = process_image(&bytes, "png", &ops).unwrap();
        let (w, h) = image_dimensions(&result).unwrap();
        assert_eq!(w, 20);
        assert_eq!(h, 20);
    }

    #[test]
    fn test_crop_in_process_image() {
        let bytes = make_test_png(100, 100);
        let ops = ImageOp {
            crop: Some((5, 5, 40, 30)),
            ..Default::default()
        };
        let (result, _) = process_image(&bytes, "png", &ops).unwrap();
        let (w, h) = image_dimensions(&result).unwrap();
        assert_eq!(w, 40);
        assert_eq!(h, 30);
    }

    #[test]
    fn test_resize_in_process_image() {
        let bytes = make_test_png(200, 100);
        let ops = ImageOp {
            resize: Some((40, 20)),
            ..Default::default()
        };
        let (result, _) = process_image(&bytes, "png", &ops).unwrap();
        let (w, h) = image_dimensions(&result).unwrap();
        assert_eq!(w, 40);
        assert_eq!(h, 20);
    }

    #[test]
    fn test_unsupported_format() {
        let result = parse_image_format("tiff");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_image_bytes() {
        let result = image_dimensions(b"not an image");
        assert!(result.is_err());
    }
}
