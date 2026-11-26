//! Compact offline EVE Online k-space stargate graph (`data/eve-kspace.bin.zst`).
//!
//! Built lazily on first use (`EveMap::global()`) so startup pays zero cost
//! when the EVE Intel add-on is off.

use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::sync::OnceLock;

static RAW_ZST: &[u8] = include_bytes!("../data/eve-kspace.bin.zst");
static GLOBAL_MAP: OnceLock<EveMap> = OnceLock::new();

pub struct EveMap {
    names: Vec<String>,
    by_lower: HashMap<String, u16>,
    adj: Vec<Vec<u16>>,
}

impl EveMap {
    /// Global lazily decoded k-space map.
    pub fn global() -> &'static Self {
        GLOBAL_MAP.get_or_init(|| Self::from_zst(RAW_ZST).expect("valid embedded eve-kspace.bin.zst"))
    }

    pub fn from_zst(packed: &[u8]) -> Result<Self, String> {
        let mut raw = Vec::new();
        ruzstd::decoding::StreamingDecoder::new(packed)
            .map_err(|e| format!("zstd init: {e}"))?
            .read_to_end(&mut raw)
            .map_err(|e| format!("zstd read: {e}"))?;
        Self::from_raw(&raw)
    }

    pub fn from_raw(raw: &[u8]) -> Result<Self, String> {
        let mut pos = 0usize;
        let read_u16 = |p: &mut usize| -> Result<u16, String> {
            let s = raw.get(*p..*p + 2).ok_or("unexpected eof (u16)")?;
            *p += 2;
            Ok(u16::from_le_bytes([s[0], s[1]]))
        };
        let read_u32 = |p: &mut usize| -> Result<u32, String> {
            let s = raw.get(*p..*p + 4).ok_or("unexpected eof (u32)")?;
            *p += 4;
            Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        };

        let n = usize::from(read_u16(&mut pos)?);
        let mut names = Vec::with_capacity(n);
        let mut by_lower = HashMap::with_capacity(n);
        for idx in 0..n {
            let len = usize::from(*raw.get(pos).ok_or("unexpected eof (name len)")?);
            pos += 1;
            let bytes = raw.get(pos..pos + len).ok_or("unexpected eof (name)")?;
            pos += len;
            let name = std::str::from_utf8(bytes).map_err(|e| format!("utf8: {e}"))?.to_string();
            by_lower.insert(name.to_ascii_lowercase(), idx as u16);
            names.push(name);
        }

        let m = read_u32(&mut pos)? as usize;
        let mut adj = vec![Vec::new(); n];
        for _ in 0..m {
            let u = read_u16(&mut pos)?;
            let v = read_u16(&mut pos)?;
            if usize::from(u) < n && usize::from(v) < n {
                adj[usize::from(u)].push(v);
                adj[usize::from(v)].push(u);
            }
        }
        Ok(Self { names, by_lower, adj })
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Exact case-insensitive system lookup. Partial/abbreviated names return `None`.
    pub fn lookup_exact(&self, token: &str) -> Option<u16> {
        let key = token.trim();
        if key.is_empty() {
            return None;
        }
        self.by_lower.get(&key.to_ascii_lowercase()).copied()
    }

    pub fn name(&self, id: u16) -> &str {
        self.names.get(usize::from(id)).map(String::as_str).unwrap_or("")
    }

    /// Shortest stargate jump count from any system in `origins` to any system
    /// in `targets`, bounded by `max_jumps` (e.g. `0..=5`).
    pub fn nearest_within(&self, origins: &[u16], targets: &[u16], max_jumps: u8) -> Option<u8> {
        if origins.is_empty() || targets.is_empty() || self.names.is_empty() {
            return None;
        }
        let mut is_target = vec![false; self.names.len()];
        for &t in targets {
            if let Some(slot) = is_target.get_mut(usize::from(t)) {
                *slot = true;
            }
        }
        let mut visited = vec![false; self.names.len()];
        let mut queue = VecDeque::new();
        for &o in origins {
            let idx = usize::from(o);
            if idx < self.names.len() && !visited[idx] {
                if is_target[idx] {
                    return Some(0);
                }
                visited[idx] = true;
                queue.push_back((o, 0u8));
            }
        }
        if max_jumps == 0 {
            return None;
        }
        while let Some((cur, d)) = queue.pop_front() {
            if d >= max_jumps {
                continue;
            }
            let nd = d + 1;
            for &nxt in &self.adj[usize::from(cur)] {
                let ni = usize::from(nxt);
                if visited[ni] {
                    continue;
                }
                if is_target[ni] {
                    return Some(nd);
                }
                visited[ni] = true;
                queue.push_back((nxt, nd));
            }
        }
        None
    }

    /// All systems reachable from `origin` within `max_jumps` stargate jumps (internal proximity helper).
    pub fn systems_within(&self, origin: u16, max_jumps: u8) -> Vec<(u16, u8)> {
        let idx = usize::from(origin);
        if idx >= self.names.len() {
            return Vec::new();
        }
        let mut visited = vec![false; self.names.len()];
        let mut queue = VecDeque::new();
        let mut out = Vec::new();
        visited[idx] = true;
        queue.push_back((origin, 0u8));
        while let Some((cur, d)) = queue.pop_front() {
            out.push((cur, d));
            if d >= max_jumps {
                continue;
            }
            let nd = d + 1;
            for &nxt in &self.adj[usize::from(cur)] {
                let ni = usize::from(nxt);
                if !visited[ni] {
                    visited[ni] = true;
                    queue.push_back((nxt, nd));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_embedded_kspace_map_and_computes_fountain_jumps() {
        let map = EveMap::global();
        assert!(map.len() > 5000);
        // Real Fountain systems from the sample chatlogs.
        let b17 = map.lookup_exact("b17o-r").expect("B17O-R exists");
        let ny6 = map.lookup_exact("NY6-FH").expect("NY6-FH exists");
        let jita = map.lookup_exact("Jita").expect("Jita exists");
        assert_eq!(map.name(b17), "B17O-R");
        // Abbreviations must NEVER match.
        assert_eq!(map.lookup_exact("B1"), None);
        assert_eq!(map.lookup_exact("C-N"), None);
        // Same system is 0 jumps; adjacent/nearby Fountain systems are within 5 jumps; Jita is far away.
        assert_eq!(map.nearest_within(&[b17], &[b17], 0), Some(0));
        assert_eq!(map.nearest_within(&[b17], &[ny6], 5), Some(3));
        assert_eq!(map.nearest_within(&[b17], &[ny6], 2), None);
        assert_eq!(map.nearest_within(&[b17], &[jita], 5), None);
        assert!(!map.systems_within(b17, 5).is_empty());
    }
}
