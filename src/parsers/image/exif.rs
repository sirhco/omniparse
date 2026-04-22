//! Shared EXIF extraction for image parsers.
//!
//! Works on any container whose EXIF payload is a standard TIFF IFD — JPEG APP1
//! `Exif\0\0`, standalone TIFF files, and WebP `EXIF` chunks.

use crate::core::MetadataValue;
use chrono::{NaiveDateTime, TimeZone, Utc};
use exif::{In, Reader, Tag, Value};
use std::io::Cursor;

/// Parse EXIF data from a byte slice and return a list of `(key, value)` pairs
/// ready to be inserted into a `Metadata` map.
///
/// Returns an empty vec (not an error) when no EXIF is present — EXIF is optional
/// metadata, not a format-correctness concern.
pub fn extract_exif_fields(data: &[u8]) -> Vec<(String, MetadataValue)> {
    let mut cursor = Cursor::new(data);
    let exif = match Reader::new().read_from_container(&mut cursor) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();

    for (tag, key) in TEXT_TAGS {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY)
            && let Some(s) = first_string(&field.value)
        {
            out.push(((*key).to_string(), MetadataValue::Text(s)));
        }
    }

    for (tag, key) in NUMBER_TAGS {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY)
            && let Some(n) = first_integer(&field.value)
        {
            out.push(((*key).to_string(), MetadataValue::Number(n)));
        }
    }

    for (tag, key) in FLOAT_TAGS {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY)
            && let Some(f) = first_rational(&field.value)
        {
            out.push(((*key).to_string(), MetadataValue::Float(f)));
        }
    }

    for (tag, key) in DATETIME_TAGS {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY)
            && let Some(s) = first_string(&field.value)
            && let Some(dt) = parse_exif_datetime(&s)
        {
            out.push(((*key).to_string(), MetadataValue::DateTime(dt)));
        }
    }

    // GPS: combine value + ref for signed decimal degrees.
    if let Some(lat) = gps_decimal(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef, b'S') {
        out.push(("gps_latitude".to_string(), MetadataValue::Float(lat)));
    }
    if let Some(lon) = gps_decimal(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef, b'W') {
        out.push(("gps_longitude".to_string(), MetadataValue::Float(lon)));
    }

    out.push(("exif_present".to_string(), MetadataValue::Boolean(true)));
    out
}

const TEXT_TAGS: &[(Tag, &str)] = &[
    (Tag::Make, "camera_make"),
    (Tag::Model, "camera_model"),
    (Tag::Software, "software"),
    (Tag::Artist, "artist"),
    (Tag::Copyright, "copyright"),
    (Tag::ImageDescription, "image_description"),
    (Tag::LensModel, "lens_model"),
];

const NUMBER_TAGS: &[(Tag, &str)] = &[
    (Tag::Orientation, "orientation"),
    (Tag::PhotographicSensitivity, "iso"),
    (Tag::PixelXDimension, "pixel_x_dimension"),
    (Tag::PixelYDimension, "pixel_y_dimension"),
];

const FLOAT_TAGS: &[(Tag, &str)] = &[
    (Tag::FocalLength, "focal_length_mm"),
    (Tag::FNumber, "f_number"),
    (Tag::ExposureTime, "exposure_time_sec"),
    (Tag::ExposureBiasValue, "exposure_bias"),
    (Tag::FocalLengthIn35mmFilm, "focal_length_35mm"),
];

const DATETIME_TAGS: &[(Tag, &str)] = &[
    (Tag::DateTimeOriginal, "datetime_original"),
    (Tag::DateTime, "datetime"),
    (Tag::DateTimeDigitized, "datetime_digitized"),
];

fn first_string(value: &Value) -> Option<String> {
    match value {
        Value::Ascii(vs) => vs
            .first()
            .map(|bytes| String::from_utf8_lossy(bytes).trim_end_matches('\0').to_string()),
        _ => Some(value.display_as(Tag::Software).to_string()),
    }
}

fn first_integer(value: &Value) -> Option<i64> {
    match value {
        Value::Byte(v) => v.first().map(|&x| x as i64),
        Value::Short(v) => v.first().map(|&x| x as i64),
        Value::Long(v) => v.first().map(|&x| x as i64),
        Value::SByte(v) => v.first().map(|&x| x as i64),
        Value::SShort(v) => v.first().map(|&x| x as i64),
        Value::SLong(v) => v.first().map(|&x| x as i64),
        _ => None,
    }
}

fn first_rational(value: &Value) -> Option<f64> {
    match value {
        Value::Rational(v) => v.first().map(|r| r.to_f64()),
        Value::SRational(v) => v.first().map(|r| r.to_f64()),
        Value::Short(v) => v.first().map(|&x| x as f64),
        Value::Long(v) => v.first().map(|&x| x as f64),
        _ => None,
    }
}

fn parse_exif_datetime(s: &str) -> Option<chrono::DateTime<Utc>> {
    // EXIF format: "YYYY:MM:DD HH:MM:SS"
    let naive = NaiveDateTime::parse_from_str(s.trim(), "%Y:%m:%d %H:%M:%S").ok()?;
    Utc.from_local_datetime(&naive).single()
}

fn gps_decimal(exif: &exif::Exif, value_tag: Tag, ref_tag: Tag, negative: u8) -> Option<f64> {
    let value_field = exif.get_field(value_tag, In::PRIMARY)?;
    let parts = match &value_field.value {
        Value::Rational(v) if v.len() >= 3 => v,
        _ => return None,
    };
    let deg = parts[0].to_f64();
    let min = parts[1].to_f64();
    let sec = parts[2].to_f64();
    let mut decimal = deg + min / 60.0 + sec / 3600.0;

    let ref_field = exif.get_field(ref_tag, In::PRIMARY);
    let is_negative = ref_field
        .and_then(|f| match &f.value {
            Value::Ascii(v) => v.first().and_then(|b| b.first()).copied(),
            _ => None,
        })
        .map(|b| b == negative)
        .unwrap_or(false);
    if is_negative {
        decimal = -decimal;
    }
    Some(decimal)
}
