//! Authorities the proxy trusts for servers besides the public ones and this
//! computer's: a company's own, or a server of the user's, put as PEM files
//! in a folder of the data directory. Read when tracewing starts.

use std::path::{Path, PathBuf};
use x509_parser::{pem::Pem, prelude::*};

/// The folder, under the data directory.
pub const FOLDER: &str = "trusted-servers";

/// An authority read from the folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authority {
    pub file: PathBuf,
    /// Its common name, or its organization, or failing both its file's name.
    pub name: String,
}

/// What the folder holds: the files that can be trusted, and those that
/// cannot, each with why.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub good: Vec<Authority>,
    pub bad: Vec<(String, String)>,
}

impl Found {
    /// The files to give the proxy.
    pub fn paths(&self) -> Vec<PathBuf> {
        self.good.iter().map(|a| a.file.clone()).collect()
    }
}

pub fn folder(data_dir: &Path) -> PathBuf {
    data_dir.join(FOLDER)
}

/// Every `.pem` and `.crt` in the folder, by name. A file the proxy could
/// not read is left out and said, so that it cannot stop tracewing starting.
pub fn read(data_dir: &Path) -> Found {
    let mut found = Found::default();
    let Ok(entries) = std::fs::read_dir(folder(data_dir)) else {
        return found;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "pem" | "crt"))
        })
        .collect();
    files.sort();
    for file in files {
        let shown = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match authority(&file) {
            Ok(name) => found.good.push(Authority {
                name: name.unwrap_or_else(|| shown.clone()),
                file,
            }),
            Err(why) => found.bad.push((shown, why)),
        }
    }
    found
}

/// The name of the first certificate in a PEM file, once every certificate
/// in it has been read.
fn authority(file: &Path) -> Result<Option<String>, String> {
    let data = std::fs::read(file).map_err(|e| e.to_string())?;
    let mut name = None;
    let mut certificates = 0;
    for pem in Pem::iter_from_buffer(&data) {
        let pem = pem.map_err(|e| format!("not PEM: {e}"))?;
        if pem.label != "CERTIFICATE" {
            continue;
        }
        let (_, cert) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| format!("not a certificate: {e}"))?;
        certificates += 1;
        if name.is_none() {
            let subject = cert.subject();
            name = subject
                .iter_common_name()
                .chain(subject.iter_organization())
                .find_map(|n| n.as_str().ok().map(str::to_owned));
        }
    }
    if certificates == 0 {
        return Err("no certificate in it".to_owned());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ca_pem(name: &str) -> String {
        let key = hudsucker::rcgen::KeyPair::generate().unwrap();
        let mut params = hudsucker::rcgen::CertificateParams::default();
        params
            .distinguished_name
            .push(hudsucker::rcgen::DnType::CommonName, name);
        params.is_ca =
            hudsucker::rcgen::IsCa::Ca(hudsucker::rcgen::BasicConstraints::Unconstrained);
        params.self_signed(&key).unwrap().pem()
    }

    #[test]
    fn good_files_are_trusted_and_bad_ones_said() {
        let dir = std::env::temp_dir().join(format!("tracewing-trusted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read(&dir), Found::default(), "no folder, nothing");
        let folder = folder(&dir);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("b-company.pem"), ca_pem("Công ty CA")).unwrap();
        std::fs::write(
            folder.join("a-two.crt"),
            ca_pem("First") + &ca_pem("Second"),
        )
        .unwrap();
        std::fs::write(
            folder.join("broken.pem"),
            "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n",
        )
        .unwrap();
        std::fs::write(folder.join("empty.pem"), "nothing here").unwrap();
        std::fs::write(folder.join("notes.txt"), "not looked at").unwrap();

        let found = read(&dir);
        let names: Vec<&str> = found.good.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["First", "Công ty CA"]);
        assert_eq!(
            found.paths(),
            [folder.join("a-two.crt"), folder.join("b-company.pem")]
        );
        let bad: Vec<&str> = found.bad.iter().map(|(file, _)| file.as_str()).collect();
        assert_eq!(bad, ["broken.pem", "empty.pem"]);
        // What is good is what the proxy can read.
        tracewing_proxy::upstream_roots(&found.paths(), false).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
