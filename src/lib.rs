#![no_std]

use core::{
    cmp::Ordering,
    convert::{
        From,
        TryInto,
    },
    fmt::Formatter,
    str::FromStr,
};

#[cfg(any(feature = "std", test))]
extern crate std;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SemVer {
    pub maj:    u16,
    pub min:    u16,
    pub rev:    u16,
    pub extra:  u16,
    pub commit: Option<u32>,
}

impl core::fmt::Display for SemVer {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "v{}.{}.{}-{}", self.maj, self.min, self.rev, self.extra)?;

        if let Some(commit) = self.commit {
            write!(f, "-g{commit:x}")?;
        }

        Ok(())
    }
}

impl SemVer {
    #[cfg(feature = "std")]
    pub fn from_git() -> Result<Self, &'static str> {
        let output = std::process::Command::new("git")
            .args(&["describe", "--tags"])
            .output()
            .map_err(|_| "failed to execute git")?;

        let gitver = output.stdout;
        let semver = core::str::from_utf8(&gitver).map_err(|_| "semver was not utf-8")?;

        FromStr::from_str(semver)
    }
}

impl FromStr for SemVer {
    type Err = &'static str;

    fn from_str(revstr: &str) -> Result<Self, &'static str> {
        let revstr = revstr.trim_end();
        let revstr = revstr.strip_prefix('v').unwrap_or(revstr);

        #[inline]
        fn parse_ver_int(s: &str) -> Result<u16, &'static str> {
            u16::from_str(s).map_err(|_| "failed to parse version number as u16")
        }

        let (maj, rest): (_, &str) = revstr.split_once('.').ok_or_else(|| "no major version")?;
        let maj = parse_ver_int(maj)?;

        let (min, rest): (_, &str) = rest.split_once('.').ok_or_else(|| "no minor version")?;
        let min = parse_ver_int(min)?;

        let patch = rest.split_once('-');
        let (patch, rest) = if let Some((patch, rest)) = patch {
            (patch, rest)
        } else {
            (rest, "")
        };
        let patch = parse_ver_int(patch)?;

        if rest.is_empty() {
            return Ok(SemVer {
                maj,
                min,
                rev: patch,
                extra: 0,
                commit: None,
            });
        }

        let (extra, commit) = if let Some((extra, commit)) = rest.split_once('-') {
            if !commit.starts_with('g') {
                return Err("invalid commit format (no 'g' prefix)");
            }

            (parse_ver_int(extra)?, Some(&commit[1..commit.len().min(9)]))
        } else {
            if let Some(commit) = rest.strip_prefix('g') {
                (0, Some(commit))
            } else {
                (parse_ver_int(rest)?, None)
            }
        };

        let commit = commit
            .map(|commit| u32::from_str_radix(commit, 16).map_err(|_| "parsing commit"))
            .transpose()?;

        Ok(SemVer {
            maj,
            min,
            rev: patch,
            extra,
            commit,
        })
    }
}

impl From<[u8; 16]> for SemVer {
    fn from(bytes: [u8; 16]) -> SemVer {
        // we use a whole word to store the `Option` flag, just to keep alignment at word alignment.
        let has_commit = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        SemVer {
            maj:    u16::from_le_bytes(bytes[0..2].try_into().unwrap()),
            min:    u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            rev:    u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
            extra:  u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
            commit: if has_commit != 0 {
                Some(u32::from_le_bytes(bytes[8..12].try_into().unwrap()))
            } else {
                None
            },
        }
    }
}

impl From<&[u8; 16]> for SemVer {
    #[inline]
    fn from(value: &[u8; 16]) -> Self {
        SemVer::from(*value)
    }
}

impl From<SemVer> for [u8; 16] {
    fn from(value: SemVer) -> Self {
        let mut ser = [0u8; 16];
        ser[0..2].copy_from_slice(&value.maj.to_le_bytes());
        ser[2..4].copy_from_slice(&value.min.to_le_bytes());
        ser[4..6].copy_from_slice(&value.rev.to_le_bytes());
        ser[6..8].copy_from_slice(&value.extra.to_le_bytes());
        ser[8..12].copy_from_slice(&value.commit.unwrap_or(0).to_le_bytes());
        ser[12..16].copy_from_slice(
            &(if value.commit.is_some() {
                1u32
            } else {
                0u32
            })
            .to_le_bytes(),
        );
        ser
    }
}

impl From<&SemVer> for [u8; 16] {
    #[inline]
    fn from(value: &SemVer) -> Self {
        value.into()
    }
}

impl PartialOrd for SemVer {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.maj
            .cmp(&other.maj)
            .then(self.min.cmp(&other.min))
            .then(self.rev.cmp(&other.rev))
            .then(self.extra.cmp(&other.extra))
    }
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use super::*;

    #[cfg(feature = "std")]
    #[test]
    fn test_gitver() {
        let gitver = SemVer::from_git().unwrap();
        std::println!("{:?}", gitver);
    }

    #[test]
    fn test_strver() {
        assert_eq!(
            SemVer::from_str("v0.9.8-760-gabcd1234"),
            Ok(SemVer {
                maj:    0,
                min:    9,
                rev:    8,
                extra:  760,
                commit: Some(0xabcd1234),
            })
        );
        assert_eq!(
            SemVer::from_str("v0.9.8-760"),
            Ok(SemVer {
                maj:    0,
                min:    9,
                rev:    8,
                extra:  760,
                commit: None,
            })
        );
        assert_eq!(
            SemVer::from_str("v0.9.8-gabcd1234"),
            Ok(SemVer {
                maj:    0,
                min:    9,
                rev:    8,
                extra:  0,
                commit: Some(0xabcd1234),
            })
        );
        let bytes: [u8; 16] = SemVer::from_str("v0.9.8-760-gabcd1234").unwrap().into();
        assert_eq!(bytes, [0, 0, 9, 0, 8, 0, 248, 2, 0x34, 0x12, 0xcd, 0xab, 0x01, 0, 0, 0]);
        let bytes: [u8; 16] = SemVer::from_str("v0.9.8-760").unwrap().into();
        assert_eq!(bytes, [0, 0, 9, 0, 8, 0, 248, 2, 0, 0, 0, 0, 0x00, 0, 0, 0]);
        let bytes: [u8; 16] = SemVer::from_str("v0.9.8-gabcd1234").unwrap().into();
        assert_eq!(bytes, [0, 0, 9, 0, 8, 0, 0, 0, 0x34, 0x12, 0xcd, 0xab, 0x01, 0, 0, 0]);
        let bytes: [u8; 16] = SemVer::from_str("v0.9.8").unwrap().into();
        assert_eq!(bytes, [0, 0, 9, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0x0, 0, 0, 0]);
        let bytes = [0, 0, 9, 0, 8, 0, 248, 2, 0x34, 0x12, 0xcd, 0xab, 0x01, 0, 0, 0];
        assert_eq!(SemVer::from_str("v0.9.8-760-gabcd1234").unwrap(), SemVer::from(bytes));
        let bytes = [
            0, 0, 9, 0, 8, 0, 248, 2, 0x34, 0x12, 0xcd,
            0xab, // these values should be ignored
            0x00, 0, 0, 0,
        ];
        assert_eq!(SemVer::from_str("v0.9.8-760").unwrap(), SemVer::from(bytes));
        assert!(
            SemVer::from_str("v0.9.8-760-gabcd1234").unwrap()
                < SemVer::from_str("v0.9.8-761-g0123456").unwrap()
        );
        assert!(
            SemVer::from_str("v0.9.8-760-gabcd1234").unwrap()
                < SemVer::from_str("v0.9.9-2").unwrap()
        );
        assert!(
            SemVer::from_str("v0.9.8-760-gabcd1234").unwrap() < SemVer::from_str("v1.0.0").unwrap()
        );
        assert!(
            SemVer::from_str("v0.9.8-760-gabcd1234").unwrap()
                != SemVer::from_str("v0.9.8-760").unwrap()
        );
        assert!(
            SemVer::from_str("v0.9.8-760-gabcd1234").unwrap()
                != SemVer::from_str("v0.9.8-760-g1234").unwrap()
        );
        assert!(
            SemVer::from_str("v0.9.8-760-gabcd1234").unwrap()
                == SemVer::from_str("v0.9.8-760-gabcd1234").unwrap()
        );
        let sv = Some(SemVer::from_str("v0.9.8-760-gabcd1234").unwrap());
        let bytes: [u8; 16] = if let Some(svb) = sv {
            svb.into()
        } else {
            [0u8; 16]
        };
        assert_eq!(bytes, [0, 0, 9, 0, 8, 0, 248, 2, 0x34, 0x12, 0xcd, 0xab, 0x01, 0, 0, 0]);
        let bytes = [
            0, 0, 9, 0, 8, 0, 248, 2, 0x34, 0x12, 0xcd,
            0xab, // these values should be ignored
            0x00, 0, 0, 0,
        ];
        assert_eq!(SemVer::from_str("v0.9.8-760").unwrap(), SemVer::from(bytes));
        assert_eq!(
            SemVer {
                maj:    0,
                min:    9,
                rev:    8,
                extra:  42,
                commit: None,
            }
            .to_string(),
            "v0.9.8-42".to_string()
        );
        assert_eq!(
            SemVer {
                maj:    0,
                min:    9,
                rev:    8,
                extra:  42,
                commit: Some(0x123abc),
            }
            .to_string(),
            "v0.9.8-42-g123abc".to_string()
        );
    }
}
