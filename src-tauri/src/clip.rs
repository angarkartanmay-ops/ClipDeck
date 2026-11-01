// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! A captured clipboard item: a list of named formats with their raw bytes,
//! exactly as the operating system offered them, so pasting is faithful.
//! Format names are the platform's own (Windows clipboard formats, macOS
//! pasteboard types, X11 targets); this module understands all of them.

use std::hash::{DefaultHasher, Hasher};
use std::io::Cursor;

use serde_json::json;

// Windows clipboard formats.
pub const TEXT: &str = "CF_UNICODETEXT";
pub const DIB: &str = "CF_DIB";
pub const HDROP: &str = "CF_HDROP";
pub const PNG: &str = "PNG";
pub const HTML: &str = "HTML Format";
pub const RTF: &str = "Rich Text Format";
#[cfg(windows)]
pub const DROP_EFFECT: &str = "Preferred DropEffect";

// macOS pasteboard types (UTIs). File URLs are stored one per line.
pub const MAC_TEXT: &str = "public.utf8-plain-text";
pub const MAC_HTML: &str = "public.html";
pub const MAC_RTF: &str = "public.rtf";
pub const MAC_PNG: &str = "public.png";
pub const MAC_TIFF: &str = "public.tiff";
pub const MAC_FILES: &str = "public.file-url";

// X11 selection targets (Linux).
pub const X_TEXT: &str = "UTF8_STRING";
pub const X_HTML: &str = "text/html";
pub const X_PNG: &str = "image/png";
pub const X_FILES: &str = "text/uri-list";

/// Formats that carry what the user actually sees; identity is based on these.
const PAYLOAD: &[&str] = &[
    TEXT, DIB, PNG, HDROP, MAC_TEXT, MAC_PNG, MAC_TIFF, MAC_FILES, X_TEXT, X_PNG, X_FILES,
];
const PLAIN_TEXT: &[&str] = &[TEXT, MAC_TEXT, X_TEXT];
const RICH: &[&str] = &[HTML, RTF, MAC_HTML, MAC_RTF, X_HTML];
const PNGS: &[&str] = &[PNG, MAC_PNG, X_PNG];

pub struct ClipFormat {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct ClipData {
    pub formats: Vec<ClipFormat>,
}

pub struct ReadOptions {
    pub images: bool,
    pub files: bool,
    pub rich: bool,
    pub max_bytes: usize,
}

impl ReadOptions {
    /// Everything, used to snapshot the clipboard before a paste.
    #[cfg(not(target_os = "linux"))]
    pub fn everything() -> Self {
        Self {
            images: true,
            files: true,
            rich: true,
            max_bytes: 512 * 1024 * 1024,
        }
    }
}

pub enum ReadOutcome {
    /// The source asked clipboard tools to ignore this content (password managers).
    Excluded,
    Empty,
    Data(ClipData),
}

impl ClipData {
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.formats
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.data.as_slice())
    }

    fn first(&self, names: &[&'static str]) -> Option<(&'static str, &[u8])> {
        names.iter().find_map(|n| self.get(n).map(|d| (*n, d)))
    }

    pub fn has_any(&self, names: &[&'static str]) -> bool {
        self.first(names).is_some()
    }

    pub fn total_size(&self) -> usize {
        self.formats.iter().map(|f| f.data.len()).sum()
    }

    pub fn text(&self) -> Option<String> {
        let (name, data) = self.first(PLAIN_TEXT)?;
        Some(if name == TEXT {
            decode_utf16(data)
        } else {
            String::from_utf8_lossy(data).into_owned()
        })
    }

    /// PNG bytes, whichever platform name they were stored under.
    pub fn png(&self) -> Option<&[u8]> {
        self.first(PNGS).map(|(_, d)| d)
    }

    /// Only the plain text format, for "paste as plain text".
    pub fn plain_text_only(&self) -> Option<ClipData> {
        let (name, data) = self.first(PLAIN_TEXT)?;
        Some(ClipData {
            formats: vec![ClipFormat {
                name: name.into(),
                data: data.to_vec(),
            }],
        })
    }

    /// Stable content hash used for de-duplication.
    pub fn hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        for f in &self.formats {
            // Rich formats often embed volatile metadata (e.g. source URLs or
            // timestamps), so identity is based on the user-visible payload.
            if PAYLOAD.contains(&f.name.as_str()) {
                hasher.write(f.name.as_bytes());
                hasher.write_usize(f.data.len());
                hasher.write(&f.data);
            }
        }
        format!("{:016x}", hasher.finish())
    }

    /// Paths of copied files, if this is a file list.
    pub fn files(&self) -> Option<Vec<String>> {
        if let Some(drop) = self.get(HDROP) {
            return Some(parse_hdrop(drop));
        }
        self.first(&[MAC_FILES, X_FILES])
            .map(|(_, list)| parse_uri_list(&String::from_utf8_lossy(list)))
    }

    // Serialized layout: "CDK1", u32 count, then per format:
    // u16 name length, name (UTF-8), u64 data length, data.
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.total_size() + 64);
        out.extend_from_slice(b"CDK1");
        out.extend_from_slice(&(self.formats.len() as u32).to_le_bytes());
        for f in &self.formats {
            out.extend_from_slice(&(f.name.len() as u16).to_le_bytes());
            out.extend_from_slice(f.name.as_bytes());
            out.extend_from_slice(&(f.data.len() as u64).to_le_bytes());
            out.extend_from_slice(&f.data);
        }
        out
    }

    pub fn deserialize(bytes: &[u8]) -> Result<ClipData, String> {
        fn take<'a>(bytes: &'a [u8], pos: &mut usize, n: usize) -> Result<&'a [u8], String> {
            let end = pos
                .checked_add(n)
                .filter(|e| *e <= bytes.len())
                .ok_or("truncated item")?;
            let slice = &bytes[*pos..end];
            *pos = end;
            Ok(slice)
        }
        let mut pos = 0;
        if take(bytes, &mut pos, 4)? != b"CDK1" {
            return Err("unknown item format".into());
        }
        let count = u32::from_le_bytes(take(bytes, &mut pos, 4)?.try_into().unwrap());
        let mut formats = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let name_len =
                u16::from_le_bytes(take(bytes, &mut pos, 2)?.try_into().unwrap()) as usize;
            let name = String::from_utf8_lossy(take(bytes, &mut pos, name_len)?).into_owned();
            let len = u64::from_le_bytes(take(bytes, &mut pos, 8)?.try_into().unwrap()) as usize;
            let data = take(bytes, &mut pos, len)?.to_vec();
            formats.push(ClipFormat { name, data });
        }
        Ok(ClipData { formats })
    }
}

pub fn decode_utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|u| *u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// What kind of item a capture is, plus what the dashboard shows for it.
pub struct Analysis {
    pub kind: &'static str,
    pub preview: String,
    pub meta: serde_json::Value,
    pub has_image: bool,
}

/// Up to this many characters of text are kept for preview and search.
const PREVIEW_CHARS: usize = 8000;

pub fn analyze(data: &ClipData) -> Analysis {
    let formats: Vec<&str> = data.formats.iter().map(|f| f.name.as_str()).collect();
    let text = data.text().unwrap_or_default();
    let preview: String = text.chars().take(PREVIEW_CHARS).collect();

    if let Some(files) = data.files().filter(|f| !f.is_empty()) {
        let names: Vec<String> = files
            .iter()
            .take(6)
            .map(|p| p.rsplit(['\\', '/']).next().unwrap_or(p).to_string())
            .collect();
        return Analysis {
            kind: "files",
            preview: files.join("\n"),
            meta: json!({ "files": names, "fileCount": files.len(), "formats": formats }),
            has_image: false,
        };
    }

    let image_dims = data
        .png()
        .and_then(png_dimensions)
        .or_else(|| data.get(DIB).and_then(dib_dimensions))
        .or_else(|| data.get(MAC_TIFF).and_then(encoded_dimensions));
    let has_text = !text.trim().is_empty();
    if let (Some((w, h)), false) = (image_dims, has_text) {
        return Analysis {
            kind: "image",
            preview: String::new(),
            meta: json!({ "width": w, "height": h, "formats": formats }),
            has_image: true,
        };
    }

    let kind = if data.has_any(RICH) { "rich" } else { "text" };
    Analysis {
        kind,
        preview,
        meta: json!({ "chars": text.chars().count(), "lines": text.lines().count(), "formats": formats }),
        has_image: false,
    }
}

fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 || &png[1..4] != b"PNG" {
        return None;
    }
    let w = u32::from_be_bytes(png[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(png[20..24].try_into().ok()?);
    Some((w, h))
}

fn dib_dimensions(dib: &[u8]) -> Option<(u32, u32)> {
    if dib.len() < 12 {
        return None;
    }
    let w = i32::from_le_bytes(dib[4..8].try_into().ok()?);
    let h = i32::from_le_bytes(dib[8..12].try_into().ok()?);
    Some((w.unsigned_abs(), h.unsigned_abs()))
}

/// Dimensions of any image format the build can decode (TIFF on macOS).
fn encoded_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// Parses a DROPFILES structure into file paths.
pub fn parse_hdrop(bytes: &[u8]) -> Vec<String> {
    if bytes.len() < 20 {
        return Vec::new();
    }
    let offset = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let wide_chars = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) != 0;
    if offset >= bytes.len() {
        return Vec::new();
    }
    let list = &bytes[offset..];
    let mut out = Vec::new();
    if wide_chars {
        let units: Vec<u16> = list
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        for part in units.split(|u| *u == 0) {
            if part.is_empty() {
                break;
            }
            out.push(String::from_utf16_lossy(part));
        }
    } else {
        for part in list.split(|b| *b == 0) {
            if part.is_empty() {
                break;
            }
            out.push(String::from_utf8_lossy(part).into_owned());
        }
    }
    out
}

/// Parses a newline-separated list of `file://` URLs (macOS and X11) into paths.
pub fn parse_uri_list(list: &str) -> Vec<String> {
    list.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.strip_prefix("file://") {
            // "file://host/path": keep the path; "file:///path" has an empty host.
            Some(rest) => percent_decode(&rest[rest.find('/').unwrap_or(0)..]),
            None => l.to_string(),
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ClipData {
        ClipData {
            formats: vec![
                ClipFormat {
                    name: TEXT.into(),
                    data: "hi".encode_utf16().flat_map(u16::to_le_bytes).collect(),
                },
                ClipFormat {
                    name: HTML.into(),
                    data: b"<b>hi</b>".to_vec(),
                },
            ],
        }
    }

    #[test]
    fn serialize_roundtrip() {
        let data = sample();
        let back = ClipData::deserialize(&data.serialize()).unwrap();
        assert_eq!(back.formats.len(), 2);
        assert_eq!(back.text().as_deref(), Some("hi"));
        assert_eq!(back.get(HTML), Some(&b"<b>hi</b>"[..]));
    }

    #[test]
    fn deserialize_rejects_garbage() {
        assert!(ClipData::deserialize(b"nope").is_err());
        assert!(ClipData::deserialize(b"CDK1\x05\0\0\0").is_err());
    }

    #[test]
    fn hash_ignores_rich_metadata() {
        let a = sample();
        let mut b = sample();
        b.formats[1].data = b"<b>hi</b><!-- other source -->".to_vec();
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn analyze_kinds() {
        assert_eq!(analyze(&sample()).kind, "rich");
        let plain = sample().plain_text_only().unwrap();
        assert_eq!(analyze(&plain).kind, "text");
    }

    #[test]
    fn utf8_platform_text() {
        let mac = ClipData {
            formats: vec![ClipFormat {
                name: MAC_TEXT.into(),
                data: "héllo".as_bytes().to_vec(),
            }],
        };
        assert_eq!(mac.text().as_deref(), Some("héllo"));
        assert_eq!(analyze(&mac).kind, "text");
    }

    #[test]
    fn hdrop_parsing() {
        let mut bytes = vec![0u8; 20];
        bytes[0] = 20;
        bytes[16] = 1;
        for path in ["C:\\a.txt", "D:\\b\\c.png"] {
            bytes.extend(path.encode_utf16().flat_map(u16::to_le_bytes));
            bytes.extend([0, 0]);
        }
        bytes.extend([0, 0]);
        assert_eq!(parse_hdrop(&bytes), vec!["C:\\a.txt", "D:\\b\\c.png"]);
    }

    #[test]
    fn uri_list_parsing() {
        let list =
            "# comment\r\nfile:///Users/me/My%20Notes.txt\nfile://host/tmp/a.png\nhttps://x.y/z\n";
        assert_eq!(
            parse_uri_list(list),
            vec!["/Users/me/My Notes.txt", "/tmp/a.png", "https://x.y/z"]
        );
        let data = ClipData {
            formats: vec![ClipFormat {
                name: X_FILES.into(),
                data: list.as_bytes().to_vec(),
            }],
        };
        assert_eq!(analyze(&data).kind, "files");
    }
}
