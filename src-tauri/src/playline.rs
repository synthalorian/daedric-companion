//! One line for the server tab: will Play reach the server?
//!
//! Same three reads as the toolkit doctor. No hash scan. Does not move files.

use serde::Serialize;
use serde_json::Value;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const REQUIRED: &str = "1.6.1170";

#[derive(Debug, Clone, Serialize)]
pub struct PlayLine {
    pub ready: bool,
    pub text: String,
}

pub fn read_play_line(skyrim_root: &str) -> PlayLine {
    let root = skyrim_root.trim();
    if root.is_empty() {
        return PlayLine {
            ready: false,
            text: "Set the Skyrim folder to know if Play will reach the server.".into(),
        };
    }
    let root = Path::new(root);
    let mut problems = Vec::new();

    let exe = find_named(root, "SkyrimSE.exe");
    let loader = find_named(root, "skse64_loader.exe").is_some();
    let version = exe.as_ref().and_then(|p| read_pe_file_version(p));
    match version {
        Some(parts) => {
            let installed = format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]);
            let got = [parts[0] as u32, parts[1] as u32, parts[2] as u32];
            let need = [1, 6, 1170];
            if got != need {
                let which = if got > need { "newer" } else { "older" };
                problems.push(format!(
                    "Skyrim is {installed}, {which} than {REQUIRED}. Play will not load the script extender."
                ));
            }
        }
        None if exe.is_none() => problems.push("SkyrimSE.exe is not in that folder.".into()),
        None => problems.push("SkyrimSE.exe has no readable FileVersion.".into()),
    }
    if !loader {
        problems.push("skse64_loader.exe is missing.".into());
    }

    if root.join("DaedricData/release-state.json.tmp").is_file()
        || dir_has_tmp_lock(&root.join("DaedricData"))
    {
        problems.push("A release-book write was left half-finished.".into());
    } else if release_is_torn(&root.join("DaedricData")) {
        problems.push("The release book is torn. State and lock do not match.".into());
    }

    let park = root.join("DaedricData/skymp-disabled/state.json");
    if std::fs::read_to_string(&park)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok())
        .is_some_and(|v| !v.is_null())
    {
        problems.push("Play-elsewhere is on. The SkyMP client is parked.".into());
    } else if dir_has_file(&root.join("DaedricData/skymp-disabled/files")) {
        problems.push("Client files are parked, but the toggle does not say so.".into());
    }

    if problems.is_empty() {
        let ver = version
            .map(|p| format!("{}.{}.{}.{}", p[0], p[1], p[2], p[3]))
            .unwrap_or_else(|| REQUIRED.into());
        PlayLine {
            ready: true,
            text: format!("Play can reach the server. Skyrim {ver}, loader present, client enabled."),
        }
    } else {
        PlayLine {
            ready: false,
            text: problems.join(" "),
        }
    }
}

fn release_is_torn(dd: &Path) -> bool {
    let state = std::fs::read_to_string(dd.join("release-state.json")).ok();
    let servers = state.and_then(|t| {
        let v: Value = serde_json::from_str(t.trim_start_matches('\u{feff}')).ok()?;
        if v.get("v")?.as_i64()? != 1 {
            return None;
        }
        Some(
            v.get("servers")?
                .as_object()?
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
        )
    });
    let Some(servers) = servers else {
        return false;
    };
    let locks = list_locks(dd);
    servers.iter().any(|s| !locks.iter().any(|n| n == &format!("release-lock.{s}.json")))
        || locks.iter().any(|n| {
            n.strip_prefix("release-lock.")
                .and_then(|r| r.strip_suffix(".json"))
                .is_none_or(|s| !servers.iter().any(|k| k == s))
        })
}

fn list_locks(dd: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dd) else {
        return Vec::new();
    };
    rd.flatten()
        .filter_map(|ent| {
            let name = ent.file_name().to_string_lossy().to_string();
            if name.starts_with("release-lock.") && name.ends_with(".json") && !name.ends_with(".tmp")
            {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

fn dir_has_tmp_lock(dd: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir(dd) else {
        return false;
    };
    rd.flatten().any(|ent| {
        let name = ent.file_name().to_string_lossy().to_string();
        name.starts_with("release-lock.") && name.ends_with(".json.tmp")
    })
}

fn dir_has_file(path: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir(path) else {
        return false;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_file() || (p.is_dir() && dir_has_file(&p)) {
            return true;
        }
    }
    false
}

fn find_named(dir: &Path, name: &str) -> Option<PathBuf> {
    let want = name.to_ascii_lowercase();
    std::fs::read_dir(dir).ok()?.flatten().find_map(|ent| {
        (ent.file_name().to_string_lossy().to_ascii_lowercase() == want).then(|| ent.path())
    })
}

fn read_pe_file_version(path: &Path) -> Option<[u16; 4]> {
    let mut file = File::open(path).ok()?;
    let dos = read_at(&mut file, 0, 64)?;
    if u16_at(&dos, 0)? != 0x5A4D {
        return None;
    }
    let pe_off = u32_at(&dos, 60)? as u64;
    let coff = read_at(&mut file, pe_off, 24)?;
    if u32_at(&coff, 0)? != 0x4550 {
        return None;
    }
    let num_sections = u16_at(&coff, 6)? as u32;
    let opt_size = u16_at(&coff, 20)? as u64;
    if opt_size < 96 {
        return None;
    }
    let opt = read_at(&mut file, pe_off + 24, opt_size)?;
    let magic = u16_at(&opt, 0)?;
    let dir_base = if magic == 0x20B { 112 } else { 96 };
    let rsrc_at = dir_base + 16;
    if rsrc_at + 8 > opt.len() {
        return None;
    }
    let rsrc_rva = u32_at(&opt, rsrc_at)?;
    if rsrc_rva == 0 {
        return None;
    }
    let sec_bytes = read_at(&mut file, pe_off + 24 + opt_size, num_sections as u64 * 40)?;
    let mut sections = Vec::new();
    let mut rsrc_section = None;
    for i in 0..num_sections as usize {
        let s = i * 40;
        let section = (
            u32_at(&sec_bytes, s + 8)?,
            u32_at(&sec_bytes, s + 12)?,
            u32_at(&sec_bytes, s + 16)?,
            u32_at(&sec_bytes, s + 20)?,
        );
        if section.1 == rsrc_rva {
            rsrc_section = Some(section);
        }
        sections.push(section);
    }
    let rsrc_section = rsrc_section?;
    if rsrc_section.2 == 0 || rsrc_section.2 > 32 * 1024 * 1024 {
        return None;
    }
    let rsrc = read_at(&mut file, rsrc_section.3 as u64, rsrc_section.2 as u64)?;
    let (type_off, type_dir) = resource_child(&rsrc, 0, Some(16))?;
    if !type_dir {
        return None;
    }
    let (name_off, name_dir) = resource_child(&rsrc, type_off, None)?;
    if !name_dir {
        return None;
    }
    let (lang_off, lang_dir) = resource_child(&rsrc, name_off, None)?;
    if lang_dir {
        return None;
    }
    let data_rva = u32_at(&rsrc, lang_off)?;
    let data_size = u32_at(&rsrc, lang_off + 4)? as u64;
    if data_size == 0 || data_size > 1_048_576 {
        return None;
    }
    let data_off = rva_to_offset(&sections, data_rva)?;
    let version = read_at(&mut file, data_off, data_size)?;
    let mut i = 0;
    while i + 52 <= version.len() {
        if u32_at(&version, i)? == 0xFEEF_04BD {
            let ms = u32_at(&version, i + 8)?;
            let ls = u32_at(&version, i + 12)?;
            return Some([
                (ms >> 16) as u16,
                (ms & 0xFFFF) as u16,
                (ls >> 16) as u16,
                (ls & 0xFFFF) as u16,
            ]);
        }
        i += 4;
    }
    None
}

fn rva_to_offset(sections: &[(u32, u32, u32, u32)], rva: u32) -> Option<u64> {
    for s in sections {
        let span = s.0.max(s.2);
        let end = s.1.saturating_add(span);
        if rva >= s.1 && rva < end {
            return Some(s.3 as u64 + (rva - s.1) as u64);
        }
    }
    None
}

fn resource_child(rsrc: &[u8], dir_offset: usize, wanted: Option<u32>) -> Option<(usize, bool)> {
    let named = u16_at(rsrc, dir_offset + 12)? as usize;
    let ids = u16_at(rsrc, dir_offset + 14)? as usize;
    let first = dir_offset + 16;
    for i in 0..(named + ids) {
        let e = first + i * 8;
        let name_or_id = u32_at(rsrc, e)?;
        let offset_to_data = u32_at(rsrc, e + 4)?;
        let is_named = (name_or_id & 0x8000_0000) != 0;
        if let Some(want) = wanted {
            if is_named || name_or_id != want {
                continue;
            }
        }
        return Some((
            (offset_to_data & 0x7FFF_FFFF) as usize,
            (offset_to_data & 0x8000_0000) != 0,
        ));
    }
    None
}

fn read_at(file: &mut File, offset: u64, len: u64) -> Option<Vec<u8>> {
    let len = usize::try_from(len).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = vec![0u8; len];
    file.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(buf.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}
