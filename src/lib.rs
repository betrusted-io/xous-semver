#[cfg(feature="std")]
use std::process::Command;
#[cfg(feature="std")]
use std::convert::{From, Into, TryInto};

#[derive(PartialEq, Eq, Debug)]
pub struct SemVer {
    pub maj: u16,
    pub min: u16,
    pub rev: u16,
    pub extra: u16,
    pub commit: Option<u32>,
}
impl SemVer {
    #[cfg(feature="std")]
    pub fn from_git() -> Result<Self, &'static str> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "git describe --tags"])
                .output()
                .map_err(|_| "failed to execute process")?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg("git describe --tags")
                .output()
                .map_err(|_| "failed to execute process")?
        };
        let gitver = output.stdout;
        let semver = String::from_utf8_lossy(&gitver);
        SemVer::from_str(&semver)
    }
    pub fn from_str(revstr: &str) -> Result<Self, &'static str> {
        let ver: Vec<&str> = revstr.trim_end().strip_prefix('v')
            .ok_or("semver does not start with 'v'!")?
            .split(['.', '-']).collect();
        if ver.len() != 4 && ver.len() != 5 && ver.len() != 3 {
            return Err("semver string has wrong number of fields");
        }
        let extra = if ver.len() == 5 {
            u16::from_str_radix(ver[3], 10).map_err(|_| "error parsing extra")?
        } else if ver.len() == 4 {
            if ver[3].strip_prefix('g').is_some() {
                0 // last string started with a 'g', it's a commit rev
            } else { // interpret last string as extra, because no leading 'g'
                u16::from_str_radix(ver[3], 10).map_err(|_| "error parsing extra")?
            }
        } else { // must be a length-3 string due to the check above
            0
        };
        Ok(SemVer {
            maj: u16::from_str_radix(ver[0], 10).map_err(|_| "error parsing maj")?,
            min: u16::from_str_radix(ver[1], 10).map_err(|_| "error parsing min")?,
            rev: u16::from_str_radix(ver[2], 10).map_err(|_| "error parsing rev")?,
            extra,
            commit: if let Some(c) = ver[ver.len() - 1].strip_prefix('g') {
                Some(
                    u32::from_str_radix(c, 16)
                    .map_err(|_| "error parsing commit")?
                )
            } else {
                None
            }
        })
    }
}
impl From::<[u8; 16]> for SemVer {
    fn from(bytes: [u8; 16]) -> SemVer {
        // we use a whole word to store the `Option` flag, just to keep alignment at word alignment.
        let has_commit = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        SemVer {
            maj: u16::from_le_bytes(bytes[0..2].try_into().unwrap()),
            min: u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            rev: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
            extra: u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
            commit: if has_commit != 0 {Some(u32::from_le_bytes(bytes[8..12].try_into().unwrap()))} else {None},
        }
    }
}
impl Into::<[u8; 16]> for SemVer {
    fn into(self) -> [u8; 16] {
        let mut ser = [0u8; 16];
        ser[0..2].copy_from_slice(&self.maj.to_le_bytes());
        ser[2..4].copy_from_slice(&self.min.to_le_bytes());
        ser[4..6].copy_from_slice(&self.rev.to_le_bytes());
        ser[6..8].copy_from_slice(&self.extra.to_le_bytes());
        ser[8..12].copy_from_slice(&self.commit.unwrap_or(0).to_le_bytes());
        ser[12..16].copy_from_slice(&(if self.commit.is_some() {1u32} else {0u32}).to_le_bytes());
        ser
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_gitver() {
        let gitver = SemVer::from_git();
        println!("{:?}", gitver);
        assert!(gitver.is_ok());
    }
    #[test]
    fn test_strver() {
        assert_eq!(
            SemVer::from_str("v0.9.8-760-gabcd1234"),
            Ok(SemVer {
                maj: 0,
                min: 9,
                rev: 8,
                extra: 760,
                commit: Some(0xabcd1234)
            })
        );
        assert_eq!(
            SemVer::from_str("v0.9.8-760"),
            Ok(SemVer {
                maj: 0,
                min: 9,
                rev: 8,
                extra: 760,
                commit: None
            })
        );
        assert_eq!(
            SemVer::from_str("v0.9.8-gabcd1234"),
            Ok(SemVer {
                maj: 0,
                min: 9,
                rev: 8,
                extra: 0,
                commit: Some(0xabcd1234)
            })
        );
        let bytes: [u8; 16] = SemVer::from_str("v0.9.8-760-gabcd1234").unwrap().into();
        assert_eq!(
            bytes,
            [0, 0,
            9, 0,
            8, 0,
            248, 2,
            0x34, 0x12, 0xcd, 0xab,
            0x01, 0, 0, 0
            ]
        );
        let bytes: [u8; 16] = SemVer::from_str("v0.9.8-760").unwrap().into();
        assert_eq!(
            bytes,
            [0, 0,
            9, 0,
            8, 0,
            248, 2,
            0, 0, 0, 0,
            0x00, 0, 0, 0
            ]
        );
        let bytes: [u8; 16] = SemVer::from_str("v0.9.8-gabcd1234").unwrap().into();
        assert_eq!(
            bytes,
            [0, 0,
            9, 0,
            8, 0,
            0, 0,
            0x34, 0x12, 0xcd, 0xab,
            0x01, 0, 0, 0
            ]
        );
        let bytes: [u8; 16] = SemVer::from_str("v0.9.8").unwrap().into();
        assert_eq!(
            bytes,
            [0, 0,
            9, 0,
            8, 0,
            0, 0,
            0, 0, 0, 0,
            0x0, 0, 0, 0
            ]
        );
        let bytes = [0, 0,
        9, 0,
        8, 0,
        248, 2,
        0x34, 0x12, 0xcd, 0xab,
        0x01, 0, 0, 0
        ];
        assert_eq!(SemVer::from_str("v0.9.8-760-gabcd1234").unwrap(),
            SemVer::from(bytes)
        );
        let bytes = [0, 0,
        9, 0,
        8, 0,
        248, 2,
        0x34, 0x12, 0xcd, 0xab, // these values should be ignored
        0x00, 0, 0, 0
        ];
        assert_eq!(SemVer::from_str("v0.9.8-760").unwrap(),
            SemVer::from(bytes)
        );
    }
}
