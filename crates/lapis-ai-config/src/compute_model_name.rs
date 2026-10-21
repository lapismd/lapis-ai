//! Bounded display-name recovery for legacy GGUF receipts without filenames.
use std::{fs::File, io::Read, path::Path};
struct Header<'a>(&'a [u8]);
impl<'a> Header<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let value = self.0.get(..n)?;
        self.0 = &self.0[n..];
        Some(value)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<&'a str> {
        let n = usize::try_from(self.u64()?).ok()?;
        std::str::from_utf8(self.take(n)?).ok()
    }
    fn skip(&mut self, kind: u32, depth: u8) -> Option<()> {
        match kind {
            0 | 1 | 7 => {
                self.take(1)?;
            }
            2 | 3 => {
                self.take(2)?;
            }
            4..=6 => {
                self.take(4)?;
            }
            10..=12 => {
                self.take(8)?;
            }
            8 => {
                self.string()?;
            }
            9 if depth < 4 => {
                let kind = self.u32()?;
                let n = self.u64()?;
                if n > 65536 {
                    return None;
                }
                for _ in 0..n {
                    self.skip(kind, depth + 1)?;
                }
            }
            _ => return None,
        }
        Some(())
    }
}
pub(super) fn read(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(65536)
        .read_to_end(&mut bytes)
        .ok()?;
    let mut h = Header(&bytes);
    if h.take(4)? != b"GGUF" || !matches!(h.u32()?, 2 | 3) {
        return None;
    }
    h.u64()?; // Tensor count; tensor data is never read.
    let count = h.u64()?;
    if count > 2048 {
        return None;
    }
    for _ in 0..count {
        let key = h.string()?;
        let kind = h.u32()?;
        if key == "general.name" && kind == 8 {
            let name = h.string()?.trim();
            return (!name.is_empty() && name.len() <= 256 && !name.chars().any(char::is_control))
                .then(|| name.to_owned());
        }
        h.skip(kind, 0)?;
    }
    None
}
