//! Smart-switch-key table -- per-app (bundle-id) input-language memory.
//!
//! Pure-Rust port of OpenKey `SmartSwitchKey.cpp`/`.h`. The C++ keeps a
//! `map<string, Int8>` plus a 1-entry read cache (`_cacheKey`/`_cacheData`);
//! both are reproduced here on [`SmartSwitchTable`]. The stored `i8` packs the
//! host convention `language | (codeTable << 1)`, but this table treats it as
//! an opaque byte -- it only stores and returns it.
//!
//! A `std::map<string, Int8>` iterates in ascending key order, so the
//! [`BTreeMap`] here makes [`to_blob`](SmartSwitchTable::to_blob) byte-exact
//! against `getSmartSwitchKeySaveData`.

use std::collections::BTreeMap;

/// The smart-switch table (`SmartSwitchKey.cpp`'s `_smartSwitchKeyData` +
/// `_cacheKey`/`_cacheData`).
#[derive(Debug, Clone)]
pub struct SmartSwitchTable {
    map: BTreeMap<String, i8>,
    /// `_cacheKey` -- last-queried bundle id. Starts empty, matching the C++
    /// static init; a query for the empty bundle id therefore hits the cache
    /// on the very first call and returns `_cacheData` (0), mirror the quirk.
    cache_key: String,
    /// `_cacheData` -- cached status for `cache_key`.
    cache_data: i8,
}

impl Default for SmartSwitchTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartSwitchTable {
    /// Empty table with an empty read cache (C++ static-init state).
    pub fn new() -> Self {
        SmartSwitchTable {
            map: BTreeMap::new(),
            cache_key: String::new(),
            cache_data: 0,
        }
    }

    /// `SmartSwitchKey.cpp:19 initSmartSwitchKey` -- load from the on-disk
    /// blob. Layout: `[u16 count]` then, per entry,
    /// `[u8 bundleLen][bundle bytes][i8 value]` (little-endian). Note this
    /// leaves the read cache untouched, exactly like the C++ (`initSmartSwitchKey`
    /// clears the map only); [`new`](Self::new)/[`from_blob`](Self::from_blob)
    /// start from an empty cache regardless.
    pub fn from_blob(data: &[u8]) -> Self {
        let mut table = SmartSwitchTable::new();
        let mut cursor = 0usize;
        let mut count: u16 = 0;
        if data.len() >= 2 {
            count = u16::from_le_bytes([data[0], data[1]]);
            cursor = 2;
        }
        for _ in 0..count {
            if cursor >= data.len() {
                break;
            }
            let bundle_len = data[cursor] as usize;
            cursor += 1;
            if cursor + bundle_len > data.len() {
                break;
            }
            let bundle = String::from_utf8_lossy(&data[cursor..cursor + bundle_len]).into_owned();
            cursor += bundle_len;
            if cursor >= data.len() {
                break;
            }
            let value = data[cursor] as i8;
            cursor += 1;
            table.map.insert(bundle, value);
        }
        table
    }

    /// `SmartSwitchKey.cpp:39 getSmartSwitchKeySaveData` -- serialize back to
    /// the blob format. Byte-exact inverse of [`from_blob`](Self::from_blob).
    pub fn to_blob(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let count = self.map.len() as u16;
        out.push(count as u8);
        out.push((count >> 8) as u8);
        for (bundle, &value) in &self.map {
            out.push(bundle.len() as u8);
            out.extend_from_slice(bundle.as_bytes());
            out.push(value as u8);
        }
        out
    }

    /// `SmartSwitchKey.cpp:54 getAppInputMethodStatus` -- the per-app language
    /// lookup. Returns:
    /// - the cached value if `bundle_id` was the last one queried;
    /// - the stored value if `bundle_id` is known (updating the cache);
    /// - `-1` if unknown -- and, as a side effect, *registers* `bundle_id`
    ///   with `current` so the next query returns it (updating the cache too).
    pub fn get_status(&mut self, bundle_id: &str, current: i32) -> i32 {
        if self.cache_key == bundle_id {
            return self.cache_data as i32;
        }
        if let Some(&value) = self.map.get(bundle_id) {
            self.cache_key = bundle_id.to_string();
            self.cache_data = value;
            return self.cache_data as i32;
        }
        self.cache_key = bundle_id.to_string();
        self.cache_data = current as i8;
        self.map.insert(bundle_id.to_string(), self.cache_data);
        -1
    }

    /// `SmartSwitchKey.cpp:69 setAppInputMethodStatus` -- set the default
    /// language for `bundle_id` and prime the read cache with it.
    pub fn set_status(&mut self, bundle_id: &str, value: i32) {
        let v = value as i8;
        self.map.insert(bundle_id.to_string(), v);
        self.cache_key = bundle_id.to_string();
        self.cache_data = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_round_trip_is_byte_exact() {
        let mut t = SmartSwitchTable::new();
        t.set_status("com.apple.Safari", 1);
        t.set_status("com.microsoft.VSCode", 0);
        t.set_status("org.mozilla.firefox", 3);
        let blob = t.to_blob();
        let t2 = SmartSwitchTable::from_blob(&blob);
        assert_eq!(blob, t2.to_blob(), "to_blob(from_blob(x)) must equal x");
    }

    #[test]
    fn get_status_miss_registers_and_returns_minus_one() {
        let mut t = SmartSwitchTable::new();
        // First query of a fresh app -> -1, registered with `current`.
        assert_eq!(t.get_status("com.foo.bar", 1), -1);
        // Same app again -> cache hit returns the registered value.
        assert_eq!(t.get_status("com.foo.bar", 0), 1);
    }

    #[test]
    fn set_then_get_returns_value() {
        let mut t = SmartSwitchTable::new();
        t.set_status("com.foo.bar", 1);
        assert_eq!(t.get_status("com.foo.bar", 0), 1);
    }

    #[test]
    fn empty_bundle_id_hits_init_cache_quirk() {
        // _cacheKey starts "" so the very first query for "" is a cache hit
        // returning _cacheData (0), NOT a -1 miss.
        let mut t = SmartSwitchTable::new();
        assert_eq!(t.get_status("", 1), 0);
    }
}
