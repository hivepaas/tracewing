//! Other devices that send their traffic here, by their address, and the names
//! given to them, kept in `devices.toml`.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    path::{Path, PathBuf},
};

/// The client's address without its port, from the `client` column, which is a
/// socket address as Rust writes it: `1.2.3.4:5`, `[::1]:5` or
/// `[::ffff:1.2.3.4]:5`. An IPv4 address mapped into IPv6 is given as IPv4.
pub const CLIENT_IP: &str = "(CASE WHEN substr(client, 1, 1) = '[' THEN \
     (CASE WHEN substr(client, 2, 7) = '::ffff:' AND instr(client, '.') > 0 \
      THEN substr(client, 9, instr(client, ']') - 9) \
      ELSE substr(client, 2, instr(client, ']') - 2) END) \
     ELSE substr(client, 1, instr(client, ':') - 1) END)";

/// Not this machine's own loopback, written as `CLIENT_IP` gives it.
pub const NOT_LOOPBACK: &str = "ip <> '' AND ip <> '::1' AND ip NOT LIKE '127.%'";

/// This machine's address on its network, as another device would reach it: the
/// one it would send from to the internet. Nothing is sent to find it.
pub fn on_the_network() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

/// The address of the device a flow came from, unless it came from this
/// machine's loopback.
pub fn device_of(client: &str) -> Option<String> {
    let ip = client.parse::<SocketAddr>().ok()?.ip().to_canonical();
    (!ip.is_loopback()).then(|| ip.to_string())
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    names: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
pub struct Devices {
    path: PathBuf,
    /// By address, as `device_of` gives it.
    names: BTreeMap<String, String>,
}

impl Devices {
    /// Names given before. A file that cannot be read gives none, and is left
    /// alone until a name is given.
    pub fn load(path: &Path) -> Self {
        let names = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str::<File>(&text).ok())
            .map(|file| file.names)
            .unwrap_or_default();
        Self {
            path: path.to_owned(),
            names,
        }
    }

    pub fn name(&self, ip: &str) -> Option<&str> {
        self.names.get(ip).map(String::as_str)
    }

    /// What to call the device at `ip`: its name, or its address.
    pub fn shown<'a>(&'a self, ip: &'a str) -> &'a str {
        self.name(ip).unwrap_or(ip)
    }

    /// Names the device at `ip`, or with an empty name forgets it, and keeps that.
    pub fn set(&mut self, ip: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            self.names.remove(ip);
        } else {
            self.names.insert(ip.to_owned(), name.to_owned());
        }
        self.save()
    }

    /// The addresses of the devices whose names have `part` in them, in any case.
    pub fn named_like(&self, part: &str) -> Vec<String> {
        let part = part.to_lowercase();
        self.names
            .iter()
            .filter(|(_, name)| name.to_lowercase().contains(&part))
            .map(|(ip, _)| ip.clone())
            .collect()
    }

    fn save(&self) -> Result<()> {
        let file = File {
            names: self.names.clone(),
        };
        let text = toml::to_string(&file)?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let new = self.path.with_extension("toml.new");
        std::fs::write(&new, text).with_context(|| format!("write {}", new.display()))?;
        std::fs::rename(&new, &self.path).with_context(|| format!("write {}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn a_device_goes_by_its_address() {
        assert_eq!(
            device_of("192.168.1.20:5000").as_deref(),
            Some("192.168.1.20")
        );
        assert_eq!(
            device_of("[::ffff:10.0.0.2]:5000").as_deref(),
            Some("10.0.0.2")
        );
        assert_eq!(device_of("[fd00::7]:5000").as_deref(), Some("fd00::7"));
        assert_eq!(device_of("127.0.0.1:5000"), None);
        assert_eq!(device_of("[::1]:5000"), None);
        assert_eq!(device_of(""), None);
    }

    #[test]
    fn the_database_reads_the_address_the_same_way() {
        let conn = Connection::open_in_memory().unwrap();
        for client in [
            "192.168.1.20:5000",
            "[::ffff:10.0.0.2]:5000",
            "[fd00::7]:5000",
            "[::1]:5000",
            "127.0.0.1:5000",
        ] {
            let ip: String = conn
                .query_row(
                    &format!("SELECT {CLIENT_IP} FROM (SELECT ?1 AS client)"),
                    [client],
                    |r| r.get(0),
                )
                .unwrap();
            let loopback: bool = conn
                .query_row(
                    &format!("SELECT NOT ({NOT_LOOPBACK}) FROM (SELECT ?1 AS ip)"),
                    [&ip],
                    |r| r.get(0),
                )
                .unwrap();
            match device_of(client) {
                Some(device) => assert_eq!((ip.as_str(), loopback), (device.as_str(), false)),
                None => assert!(loopback, "{client} is this machine"),
            }
        }
    }

    #[test]
    fn names_are_kept_and_forgotten() {
        let dir = std::env::temp_dir().join(format!("devices-{}", std::process::id()));
        let path = dir.join("devices.toml");
        let mut devices = Devices::load(&path);
        assert_eq!(devices.shown("192.168.1.20"), "192.168.1.20");
        devices.set("192.168.1.20", " iPhone test ").unwrap();
        devices.set("192.168.1.21", "Pixel").unwrap();
        let devices = Devices::load(&path);
        assert_eq!(devices.name("192.168.1.20"), Some("iPhone test"));
        assert_eq!(
            devices.named_like("IPHONE"),
            vec!["192.168.1.20".to_owned()]
        );
        let mut devices = devices;
        devices.set("192.168.1.20", "").unwrap();
        assert_eq!(Devices::load(&path).name("192.168.1.20"), None);
        assert_eq!(Devices::load(&path).name("192.168.1.21"), Some("Pixel"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
