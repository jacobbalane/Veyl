//! DNS-SD records are hints. A connection still authenticates the full paired identity.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::NativeRequest;

use serde::Serialize;
use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddrV4},
    time::Instant,
};

#[derive(Clone, Debug)]
pub enum DiscoveryEvent {
    Instance { name: String, gone: bool },
    Resolved(Service),
    Failed(u32),
}

#[derive(Clone, Debug)]
pub struct Service {
    pub instance: String,
    pub address: SocketAddrV4,
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NearbyPhone {
    pub id: String,
    pub name: String,
    pub model: String,
    pub address: String,
}

struct SeenPhone {
    phone: NearbyPhone,
    instance: String,
    at: Instant,
}

#[derive(Default)]
pub struct NearbyTable(BTreeMap<String, SeenPhone>);
impl NearbyTable {
    pub fn update(&mut self, service: Service, now: Instant) -> bool {
        let Some(phone) = service.phone() else {
            return false;
        };
        if !self.0.contains_key(&phone.id) && self.0.len() >= 128 {
            return false;
        }
        let changed = self.0.get(&phone.id).is_none_or(|old| old.phone != phone);
        self.0.insert(
            phone.id.clone(),
            SeenPhone {
                phone,
                instance: service.instance,
                at: now,
            },
        );
        changed
    }
    pub fn remove_instance(&mut self, instance: &str) -> bool {
        let before = self.0.len();
        self.0
            .retain(|_, seen| !seen.instance.eq_ignore_ascii_case(instance));
        before != self.0.len()
    }
    pub fn expire(&mut self, now: Instant) -> bool {
        let before = self.0.len();
        self.0
            .retain(|_, seen| now.saturating_duration_since(seen.at).as_secs() < 90);
        before != self.0.len()
    }
    pub fn phones(&self) -> Vec<NearbyPhone> {
        self.0.values().map(|seen| seen.phone.clone()).collect()
    }
}
impl Service {
    fn phone(&self) -> Option<NearbyPhone> {
        let get = |key| self.properties.get(key).map(String::as_str);
        if get("v")? != "1" || self.address.port() == 0 || !unicast(*self.address.ip()) {
            return None;
        }
        let id = get("id")?;
        if id.len() != 8 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let name = get("name")?;
        let model = get("model")?;
        if [name, model]
            .iter()
            .any(|s| s.len() > 40 || s.chars().any(char::is_control))
        {
            return None;
        }
        Some(NearbyPhone {
            id: id.to_ascii_lowercase(),
            name: name.into(),
            model: model.into(),
            address: self.address.to_string(),
        })
    }
}
fn unicast(ip: Ipv4Addr) -> bool {
    !ip.is_unspecified() && !ip.is_multicast() && ip != Ipv4Addr::BROADCAST
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn service() -> Service {
        Service {
            instance: "Test._veyl._tcp.local".into(),
            address: "192.168.1.2:47820".parse().unwrap(),
            properties: [
                ("v", "1"),
                ("id", "A0b1c2d3"),
                ("name", "Desk phone"),
                ("model", "Test"),
            ]
            .map(|(k, v)| (k.into(), v.into()))
            .into(),
        }
    }
    #[test]
    fn refreshes_address_coalesces_identity_and_expires_at_ninety_seconds() {
        let now = Instant::now();
        let mut table = NearbyTable::default();
        assert!(table.update(service(), now));
        assert!(!table.update(service(), now + Duration::from_secs(30)));
        let mut moved = service();
        moved.address.set_ip("192.168.1.3".parse().unwrap());
        assert!(table.update(moved, now + Duration::from_secs(60)));
        assert_eq!(table.phones()[0].id, "a0b1c2d3");
        assert_eq!(table.phones().len(), 1);
        assert!(!table.expire(now + Duration::from_secs(149)));
        assert!(table.expire(now + Duration::from_secs(150)));
        assert!(table.phones().is_empty());
        table.update(service(), now);
        assert!(table.remove_instance("test._veyl._tcp.local"));
    }
    #[test]
    fn ignores_unsupported_malformed_and_nonunicast_advertisements() {
        for (key, value) in [
            ("v", "2"),
            ("id", "abc"),
            ("id", "abcdefgh"),
            ("name", "bad\nname"),
            ("model", "A very long model name that exceeds forty bytes"),
        ] {
            let mut invalid = service();
            invalid.properties.insert(key.into(), value.into());
            assert!(invalid.phone().is_none());
        }
        for address in [
            "0.0.0.0:1",
            "224.0.0.251:1",
            "255.255.255.255:1",
            "192.168.1.2:0",
        ] {
            let mut invalid = service();
            invalid.address = address.parse().unwrap();
            assert!(invalid.phone().is_none());
        }
    }
}
