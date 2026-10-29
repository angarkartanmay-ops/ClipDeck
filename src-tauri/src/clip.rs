// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Reading and writing the Windows clipboard, and the on-disk format of a
//! captured item (a list of named clipboard formats with their raw bytes).

use std::hash::{DefaultHasher, Hasher};
use std::time::Duration;

use serde_json::json;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};

pub const TEXT: &str = "CF_UNICODETEXT";
pub const DIB: &str = "CF_DIB";
pub const HDROP: &str = "CF_HDROP";
pub const PNG: &str = "PNG";
pub const HTML: &str = "HTML Format";
pub const RTF: &str = "Rich Text Format";
pub const DROP_EFFECT: &str = "Preferred DropEffect";

const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;

pub struct ClipFormat {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct ClipData {
    pub formats: Vec<ClipFormat>,
}

impl ClipData {
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.formats
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.data.as_slice())
    }

    pub fn has(&self, name: &str) -> bool {
        self.formats.iter().any(|f| f.name == name)
    }

    pub fn total_size(&self) -> usize {
        self.formats.iter().map(|f| f.data.len()).sum()
    }

    pub fn text(&self) -> Option<String> {
        self.get(TEXT).map(decode_utf16)
    }

    /// Only the plain text format, for "paste as plain text".
    pub fn plain_text_only(&self) -> Option<ClipData> {
        let data = self.get(TEXT)?.to_vec();
        Some(ClipData {
            formats: vec![ClipFormat {
                name: TEXT.into(),
                data,
            }],
        })
    }

    /// Stable content hash used for de-duplication.
    pub fn hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        for f in &self.formats {
            // Rich formats often embed volatile metadata (e.g. source URLs or
            // timestamps), so identity is based on the user-visible payload.
            if matches!(f.name.as_str(), TEXT | DIB | PNG | HDROP) {
                hasher.write(f.name.as_bytes());
                hasher.write_usize(f.data.len());
                hasher.write(&f.data);
            }
        }
        format!("{:016x}", hasher.finish())
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

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn format_id(name: &str) -> u32 {
    match name {
        TEXT => CF_UNICODETEXT,
        DIB => CF_DIB,
        HDROP => CF_HDROP,
        other => {
            let w = wide(other);
            unsafe { RegisterClipboardFormatW(PCWSTR(w.as_ptr())) }
        }
    }
}

/// Keeps the clipboard open for the lifetime of the guard.
struct OpenGuard;

impl Drop for OpenGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

/// Another application may briefly hold the clipboard; retry for ~300 ms.
fn open(owner: HWND) -> Result<OpenGuard, String> {
    for attempt in 0..20 {
        if unsafe { OpenClipboard(Some(owner)) }.is_ok() {
            return Ok(OpenGuard);
        }
        std::thread::sleep(Duration::from_millis(if attempt < 5 { 5 } else { 20 }));
    }
    Err("clipboard is locked by another application".into())
}

fn available(id: u32) -> bool {
    id != 0 && unsafe { IsClipboardFormatAvailable(id) }.is_ok()
}

/// Copies one format out of the open clipboard. Returns `None` when the
/// format is missing or larger than `max_bytes`.
fn read_format(id: u32, max_bytes: usize) -> Option<Vec<u8>> {
    if !available(id) {
        return None;
    }
    unsafe {
        let handle = GetClipboardData(id).ok()?;
        let hglobal = HGLOBAL(handle.0);
        let size = GlobalSize(hglobal);
        if size == 0 || size > max_bytes {
            return None;
        }
        let ptr = GlobalLock(hglobal) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let data = std::slice::from_raw_parts(ptr, size).to_vec();
        let _ = GlobalUnlock(hglobal);
        Some(data)
    }
}

pub struct ReadOptions {
    pub images: bool,
    pub files: bool,
    pub rich: bool,
    pub max_bytes: usize,
}

impl ReadOptions {
    /// Everything, used to snapshot the clipboard before a paste.
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
    /// The source asked monitors to ignore this content (password managers).
    Excluded,
    Empty,
    Data(ClipData),
}

pub fn read(owner: HWND, opts: &ReadOptions) -> Result<ReadOutcome, String> {
    let _guard = open(owner)?;

    // Honour the documented opt-out formats used by password managers.
    if available(format_id("ExcludeClipboardContentFromMonitorProcessing"))
        || available(format_id("Clipboard Viewer Ignore"))
    {
        return Ok(ReadOutcome::Excluded);
    }
    if let Some(flag) = read_format(format_id("CanIncludeInClipboardHistory"), 64) {
        if flag.len() >= 4 && u32::from_le_bytes([flag[0], flag[1], flag[2], flag[3]]) == 0 {
            return Ok(ReadOutcome::Excluded);
        }
    }

    let mut budget = opts.max_bytes;
    let mut formats = Vec::new();
    let mut push = |name: &str, formats: &mut Vec<ClipFormat>| {
        if let Some(mut data) = read_format(format_id(name), budget) {
            if name == TEXT {
                let units = data.chunks_exact(2).take_while(|c| c != &[0, 0]).count();
                data.truncate(units * 2);
                if data.is_empty() {
                    return;
                }
            }
            budget = budget.saturating_sub(data.len());
            formats.push(ClipFormat {
                name: name.into(),
                data,
            });
        }
    };

    if available(CF_HDROP) {
        if !opts.files {
            return Ok(ReadOutcome::Empty);
        }
        push(HDROP, &mut formats);
        push(DROP_EFFECT, &mut formats);
        push(TEXT, &mut formats);
    } else {
        push(TEXT, &mut formats);
        if opts.rich {
            push(HTML, &mut formats);
            push(RTF, &mut formats);
        }
        if opts.images {
            push(PNG, &mut formats);
            push(DIB, &mut formats);
        }
    }

    let has_text = formats
        .iter()
        .any(|f| f.name == TEXT && !decode_utf16(&f.data).trim().is_empty());
    let has_payload = has_text
        || formats
            .iter()
            .any(|f| matches!(f.name.as_str(), DIB | PNG | HDROP));
    if !has_payload {
        return Ok(ReadOutcome::Empty);
    }
    Ok(ReadOutcome::Data(ClipData { formats }))
}

pub fn write(owner: HWND, data: &ClipData) -> Result<(), String> {
    let _guard = open(owner)?;
    unsafe {
        EmptyClipboard().map_err(|e| format!("EmptyClipboard: {e}"))?;
        for f in &data.formats {
            // Text formats are stored without terminators; add them back.
            let terminator: &[u8] = match f.name.as_str() {
                TEXT => &[0, 0],
                _ => &[],
            };
            let len = f.data.len() + terminator.len();
            let Ok(hglobal) = GlobalAlloc(GMEM_MOVEABLE, len.max(1)) else {
                continue;
            };
            let ptr = GlobalLock(hglobal) as *mut u8;
            if ptr.is_null() {
                let _ = GlobalFree(Some(hglobal));
                continue;
            }
            std::ptr::copy_nonoverlapping(f.data.as_ptr(), ptr, f.data.len());
            std::ptr::copy_nonoverlapping(
                terminator.as_ptr(),
                ptr.add(f.data.len()),
                terminator.len(),
            );
            let _ = GlobalUnlock(hglobal);
            if SetClipboardData(format_id(&f.name), Some(HANDLE(hglobal.0))).is_err() {
                let _ = GlobalFree(Some(hglobal));
            }
        }
    }
    Ok(())
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
    let image_dims = data
        .get(PNG)
        .and_then(png_dimensions)
        .or_else(|| data.get(DIB).and_then(dib_dimensions));

    if let Some(drop) = data.get(HDROP) {
        let files = parse_hdrop(drop);
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

    let has_text = !text.trim().is_empty();
    if let (Some((w, h)), false) = (image_dims, has_text) {
        return Analysis {
            kind: "image",
            preview: String::new(),
            meta: json!({ "width": w, "height": h, "formats": formats }),
            has_image: true,
        };
    }

    let kind = if data.has(HTML) || data.has(RTF) {
        "rich"
    } else {
        "text"
    };
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
}
