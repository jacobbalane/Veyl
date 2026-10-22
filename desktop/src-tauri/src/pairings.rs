//! Only DPAPI ciphertext reaches disk. Views sent to the interface cannot contain a link key.
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read, Write},
    net::SocketAddrV4,
    path::{Path, PathBuf},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use veyl_link::{
    crypto::Secret,
    pairing::{PairedPeer, Peer},
};
use zeroize::Zeroizing;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPhone {
    pub phone_id: String,
    pub phone_name: String,
    pub model: String,
    pub paired_at: String,
    pub last_seen: String,
    pub last_address: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    phone_id: String,
    phone_name: String,
    model: String,
    k_link_dpapi: String,
    paired_at: String,
    last_seen: String,
    last_address: String,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}
impl Entry {
    fn view(&self) -> SavedPhone {
        SavedPhone {
            phone_id: self.phone_id.clone(),
            phone_name: self.phone_name.clone(),
            model: self.model.clone(),
            paired_at: self.paired_at.clone(),
            last_seen: self.last_seen.clone(),
            last_address: self.last_address.clone(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Document {
    version: u32,
    phones: Vec<Entry>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}
pub struct PairingStore {
    path: PathBuf,
    pc_id: [u8; 16],
    document: Document,
}
impl PairingStore {
    pub fn load(path: &Path, pc_id: [u8; 16]) -> io::Result<Self> {
        let document: Document = match fs::File::open(path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(1_048_577).read_to_end(&mut bytes)?;
                if bytes.len() > 1_048_576 {
                    return Err(invalid());
                }
                serde_json::from_slice(&bytes).map_err(|_| invalid())?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Document {
                version: 1,
                phones: vec![],
                extra: BTreeMap::new(),
            },
            Err(error) => return Err(error),
        };
        let mut ids = BTreeSet::new();
        if document.version != 1 || document.phones.len() > 64 {
            return Err(invalid());
        }
        for entry in &document.phones {
            id_bytes(&entry.phone_id)?;
            if !ids.insert(&entry.phone_id)
                || entry.phone_name.len() > 40
                || entry.model.len() > 40
                || STANDARD
                    .decode(&entry.k_link_dpapi)
                    .map_or(true, |bytes| bytes.is_empty() || bytes.len() > 8192)
                || entry.last_address.parse::<SocketAddrV4>().is_err()
                || OffsetDateTime::parse(&entry.paired_at, &Rfc3339).is_err()
                || OffsetDateTime::parse(&entry.last_seen, &Rfc3339).is_err()
            {
                return Err(invalid());
            }
        }
        Ok(Self {
            path: path.into(),
            pc_id,
            document,
        })
    }
    pub fn list(&self) -> Vec<SavedPhone> {
        self.document.phones.iter().map(Entry::view).collect()
    }
    pub fn find(&self, id: &str) -> io::Result<Option<PairedPeer>> {
        let Some(entry) = self
            .document
            .phones
            .iter()
            .find(|entry| entry.phone_id == id)
        else {
            return Ok(None);
        };
        let phone_id = id_bytes(id)?;
        let wrapped = STANDARD
            .decode(&entry.k_link_dpapi)
            .map_err(|_| invalid())?;
        let plain = unprotect(&entropy(self.pc_id, phone_id), &wrapped)?;
        let key: Zeroizing<[u8; 32]> =
            Zeroizing::new(plain.as_slice().try_into().map_err(|_| invalid())?);
        Ok(Some(PairedPeer {
            peer: Peer {
                id: phone_id,
                name: entry.phone_name.clone(),
                model: entry.model.clone(),
            },
            link: Secret::new(*key),
        }))
    }
    pub fn remember(&mut self, peer: &PairedPeer, address: SocketAddrV4) -> io::Result<()> {
        if peer.peer.name.len() > 40 || peer.peer.model.len() > 40 {
            return Err(invalid());
        }
        let id = URL_SAFE_NO_PAD.encode(peer.peer.id);
        let wrapped = protect(&entropy(self.pc_id, peer.peer.id), peer.link.expose())?;
        let mut document = self.document.clone();
        let previous = document
            .phones
            .iter()
            .find(|entry| entry.phone_id == id)
            .cloned();
        document.phones.retain(|entry| entry.phone_id != id);
        if document.phones.len() >= 64 {
            return Err(invalid());
        }
        let now = timestamp();
        document.phones.push(Entry {
            phone_id: id,
            phone_name: peer.peer.name.clone(),
            model: peer.peer.model.clone(),
            k_link_dpapi: STANDARD.encode(wrapped),
            paired_at: previous
                .as_ref()
                .map(|entry| entry.paired_at.clone())
                .unwrap_or_else(|| now.clone()),
            last_seen: now,
            last_address: address.to_string(),
            extra: previous.map(|entry| entry.extra).unwrap_or_default(),
        });
        self.persist(document)
    }
    pub fn seen(&mut self, id: &str, address: SocketAddrV4, name: Option<&str>) -> io::Result<()> {
        let mut document = self.document.clone();
        let Some(entry) = document
            .phones
            .iter_mut()
            .find(|entry| entry.phone_id == id)
        else {
            return Ok(());
        };
        entry.last_seen = timestamp();
        entry.last_address = address.to_string();
        if let Some(name) = name {
            if name.len() > 40 {
                return Err(invalid());
            }
            entry.phone_name = name.into();
        }
        self.persist(document)
    }
    pub fn forget(&mut self, id: &str) -> io::Result<()> {
        let mut document = self.document.clone();
        document.phones.retain(|entry| entry.phone_id != id);
        self.persist(document)
    }
    fn persist(&mut self, document: Document) -> io::Result<()> {
        let directory = self.path.parent().ok_or_else(invalid)?;
        fs::create_dir_all(directory)?;
        let mut file = tempfile::NamedTempFile::new_in(directory)?;
        serde_json::to_writer_pretty(&mut file, &document).map_err(io::Error::other)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|error| error.error)?;
        self.document = document;
        Ok(())
    }
}
pub fn id_bytes(id: &str) -> io::Result<[u8; 16]> {
    let bytes = URL_SAFE_NO_PAD.decode(id).map_err(|_| invalid())?;
    if URL_SAFE_NO_PAD.encode(&bytes) != id {
        return Err(invalid());
    }
    bytes.try_into().map_err(|_| invalid())
}
pub fn short_id(id: &str) -> io::Result<String> {
    Ok(id_bytes(id)?[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
fn timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("UTC timestamp")
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid Veyl pairing store.")
}
fn entropy(pc: [u8; 16], phone: [u8; 16]) -> Vec<u8> {
    [
        b"veyl desktop pairing v1".as_slice(),
        pc.as_slice(),
        phone.as_slice(),
    ]
    .concat()
}

#[cfg(windows)]
fn protect(entropy: &[u8], plain: &[u8]) -> io::Result<Vec<u8>> {
    Ok(dpapi(entropy, plain, true)?.to_vec())
}
#[cfg(windows)]
fn unprotect(entropy: &[u8], wrapped: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    dpapi(entropy, wrapped, false)
}
#[cfg(not(windows))]
fn protect(_: &[u8], _: &[u8]) -> io::Result<Vec<u8>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Windows DPAPI is required.",
    ))
}
#[cfg(not(windows))]
fn unprotect(_: &[u8], _: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Windows DPAPI is required.",
    ))
}

#[cfg(windows)]
fn dpapi(entropy: &[u8], input: &[u8], protect: bool) -> io::Result<Zeroizing<Vec<u8>>> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    use zeroize::Zeroize;
    if input.len() > 8192 {
        return Err(invalid());
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr().cast_mut(),
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: entropy.len() as u32,
        pbData: entropy.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // DPAPI runs as the current user. UI and machine-wide protection are never requested.
    let succeeded = unsafe {
        if protect {
            CryptProtectData(
                &input,
                std::ptr::null(),
                &entropy,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                &entropy,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if succeeded == 0 {
        return Err(io::Error::last_os_error());
    }
    if output.pbData.is_null() {
        return Err(invalid());
    }
    // The platform allocation is always scrubbed and freed, including rejected output lengths.
    unsafe {
        let bytes = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let result = if bytes.len() <= 8192 {
            Ok(Zeroizing::new(bytes.to_vec()))
        } else {
            Err(invalid())
        };
        bytes.zeroize();
        LocalFree(output.pbData.cast());
        result
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn peer() -> PairedPeer {
        PairedPeer {
            peer: Peer {
                id: [7; 16],
                name: "Desk phone".into(),
                model: "Test".into(),
            },
            link: Secret::new([3; 32]),
        }
    }
    #[test]
    fn native_dpapi_survives_restart_binds_both_identities_and_keeps_unknown_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pairings.json");
        let pc = [8; 16];
        let peer = peer();
        let id = URL_SAFE_NO_PAD.encode(peer.peer.id);
        let mut store = PairingStore::load(&path, pc).unwrap();
        store
            .remember(&peer, "127.0.0.1:47820".parse().unwrap())
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains(&URL_SAFE_NO_PAD.encode(peer.link.expose())));
        let mut data: Value = serde_json::from_str(&text).unwrap();
        data["future"] = 42.into();
        data["phones"][0]["future_phone"] = true.into();
        fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
        let mut restarted = PairingStore::load(&path, pc).unwrap();
        assert!(restarted.find(&id).unwrap().unwrap().link.expose() == peer.link.expose());
        assert!(PairingStore::load(&path, [9; 16])
            .unwrap()
            .find(&id)
            .is_err());
        restarted
            .seen(&id, "127.0.0.1:47821".parse().unwrap(), Some("Renamed"))
            .unwrap();
        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["future"], 42);
        assert_eq!(saved["phones"][0]["future_phone"], true);
        assert_eq!(restarted.list()[0].phone_name, "Renamed");
        let mut swapped = saved;
        let other = URL_SAFE_NO_PAD.encode([6; 16]);
        swapped["phones"][0]["phone_id"] = other.clone().into();
        fs::write(&path, serde_json::to_vec(&swapped).unwrap()).unwrap();
        assert!(PairingStore::load(&path, pc).unwrap().find(&other).is_err());
        restarted.forget(&id).unwrap();
        assert!(PairingStore::load(&path, pc).unwrap().list().is_empty());
    }
    #[test]
    fn failed_save_keeps_memory_and_malformed_store_is_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pairings.json");
        let mut store = PairingStore::load(&path, [8; 16]).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(store
            .remember(&peer(), "127.0.0.1:1".parse().unwrap())
            .is_err());
        assert!(store.list().is_empty());
        fs::remove_dir(&path).unwrap();
        fs::write(&path, b"invalid").unwrap();
        assert!(PairingStore::load(&path, [8; 16]).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"invalid");
    }
}
