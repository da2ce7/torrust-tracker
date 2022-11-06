use aquatic_udp_protocol::{AnnounceEvent, NumberOfBytes};
use serde::{Deserialize, Serialize};

pub const MAX_SCRAPE_TORRENTS: u8 = 74;
pub const AUTH_KEY_LENGTH: usize = 32;

#[repr(u32)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Actions {
    Connect = 0,
    Announce = 1,
    Scrape = 2,
    Error = 3,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "AnnounceEvent")]
pub enum AnnounceEventDef {
    Started,
    Stopped,
    Completed,
    None,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "NumberOfBytes")]
pub struct NumberOfBytesDef(pub i64);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct InfoHash(pub [u8; 20]);

impl std::fmt::Display for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut chars = [0u8; 40];
        binascii::bin2hex(&self.0, &mut chars).expect("failed to hexlify");
        write!(f, "{}", std::str::from_utf8(&chars).unwrap())
    }
}

impl std::str::FromStr for InfoHash {
    type Err = binascii::ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut i = Self([0u8; 20]);
        if s.len() != 40 {
            return Err(binascii::ConvertError::InvalidInputLength);
        }
        binascii::hex2bin(s.as_bytes(), &mut i.0)?;
        Ok(i)
    }
}

impl std::cmp::PartialOrd<InfoHash> for InfoHash {
    fn partial_cmp(&self, other: &InfoHash) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for InfoHash {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl std::convert::From<&[u8]> for InfoHash {
    fn from(data: &[u8]) -> InfoHash {
        assert_eq!(data.len(), 20);
        let mut ret = InfoHash([0u8; 20]);
        ret.0.clone_from_slice(data);
        ret
    }
}

impl From<[u8; 20]> for InfoHash {
    fn from(val: [u8; 20]) -> Self {
        InfoHash(val)
    }
}

impl serde::ser::Serialize for InfoHash {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut buffer = [0u8; 40];
        let bytes_out = binascii::bin2hex(&self.0, &mut buffer).ok().unwrap();
        let str_out = std::str::from_utf8(bytes_out).unwrap();
        serializer.serialize_str(str_out)
    }
}

impl<'de> serde::de::Deserialize<'de> for InfoHash {
    fn deserialize<D: serde::de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        des.deserialize_str(InfoHashVisitor)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::InfoHash;

    #[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
    struct ContainingInfoHash {
        pub info_hash: InfoHash,
    }

    #[test]
    fn an_info_hash_can_be_created_from_a_valid_40_utf8_char_string_representing_an_hexadecimal_value() {
        let info_hash = InfoHash::from_str("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        assert!(info_hash.is_ok());
    }

    #[test]
    fn an_info_hash_can_not_be_created_from_a_utf8_string_representing_a_not_valid_hexadecimal_value() {
        let info_hash = InfoHash::from_str("GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG");
        assert!(info_hash.is_err());
    }

    #[test]
    fn an_info_hash_can_only_be_created_from_a_40_utf8_char_string() {
        let info_hash = InfoHash::from_str(&"F".repeat(39));
        assert!(info_hash.is_err());

        let info_hash = InfoHash::from_str(&"F".repeat(41));
        assert!(info_hash.is_err());
    }

    #[test]
    fn an_info_hash_should_by_displayed_like_a_40_utf8_lowercased_char_hex_string() {
        let info_hash = InfoHash::from_str("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap();

        let output = format!("{}", info_hash);

        assert_eq!(output, "ffffffffffffffffffffffffffffffffffffffff");
    }

    #[test]
    fn an_info_hash_can_be_created_from_a_valid_20_byte_array_slice() {
        let info_hash: InfoHash = [255u8; 20].as_slice().into();

        assert_eq!(
            info_hash,
            InfoHash::from_str("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap()
        );
    }

    #[test]
    fn an_info_hash_can_be_created_from_a_valid_20_byte_array() {
        let info_hash: InfoHash = [255u8; 20].into();

        assert_eq!(
            info_hash,
            InfoHash::from_str("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap()
        );
    }

    #[test]
    fn an_info_hash_can_be_serialized() {
        let s = ContainingInfoHash {
            info_hash: InfoHash::from_str("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap(),
        };

        let json_serialized_value = serde_json::to_string(&s).unwrap();

        assert_eq!(
            json_serialized_value,
            r#"{"info_hash":"ffffffffffffffffffffffffffffffffffffffff"}"#
        );
    }

    #[test]
    fn an_info_hash_can_be_deserialized() {
        let json = json!({
            "info_hash": "ffffffffffffffffffffffffffffffffffffffff",
        });

        let s: ContainingInfoHash = serde_json::from_value(json).unwrap();

        assert_eq!(
            s,
            ContainingInfoHash {
                info_hash: InfoHash::from_str("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap()
            }
        );
    }
}

struct InfoHashVisitor;

impl<'v> serde::de::Visitor<'v> for InfoHashVisitor {
    type Value = InfoHash;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a 40 character long hash")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v.len() != 40 {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a 40 character long string",
            ));
        }

        let mut res = InfoHash([0u8; 20]);

        if binascii::hex2bin(v.as_bytes(), &mut res.0).is_err() {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"expected a hexadecimal string",
            ));
        } else {
            Ok(res)
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
pub struct PeerId(pub [u8; 20]);

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = [0u8; 20];
        let bytes_out = binascii::bin2hex(&self.0, &mut buffer).ok();
        match bytes_out {
            Some(bytes) => write!(f, "{}", std::str::from_utf8(bytes).unwrap()),
            None => write!(f, ""),
        }
    }
}

impl PeerId {
    pub fn get_client_name(&self) -> Option<&'static str> {
        if self.0[0] == b'M' {
            return Some("BitTorrent");
        }
        if self.0[0] == b'-' {
            let name = match &self.0[1..3] {
                b"AG" => "Ares",
                b"A~" => "Ares",
                b"AR" => "Arctic",
                b"AV" => "Avicora",
                b"AX" => "BitPump",
                b"AZ" => "Azureus",
                b"BB" => "BitBuddy",
                b"BC" => "BitComet",
                b"BF" => "Bitflu",
                b"BG" => "BTG (uses Rasterbar libtorrent)",
                b"BR" => "BitRocket",
                b"BS" => "BTSlave",
                b"BX" => "~Bittorrent X",
                b"CD" => "Enhanced CTorrent",
                b"CT" => "CTorrent",
                b"DE" => "DelugeTorrent",
                b"DP" => "Propagate Data Client",
                b"EB" => "EBit",
                b"ES" => "electric sheep",
                b"FT" => "FoxTorrent",
                b"FW" => "FrostWire",
                b"FX" => "Freebox BitTorrent",
                b"GS" => "GSTorrent",
                b"HL" => "Halite",
                b"HN" => "Hydranode",
                b"KG" => "KGet",
                b"KT" => "KTorrent",
                b"LH" => "LH-ABC",
                b"LP" => "Lphant",
                b"LT" => "libtorrent",
                b"lt" => "libTorrent",
                b"LW" => "LimeWire",
                b"MO" => "MonoTorrent",
                b"MP" => "MooPolice",
                b"MR" => "Miro",
                b"MT" => "MoonlightTorrent",
                b"NX" => "Net Transport",
                b"PD" => "Pando",
                b"qB" => "qBittorrent",
                b"QD" => "QQDownload",
                b"QT" => "Qt 4 Torrent example",
                b"RT" => "Retriever",
                b"S~" => "Shareaza alpha/beta",
                b"SB" => "~Swiftbit",
                b"SS" => "SwarmScope",
                b"ST" => "SymTorrent",
                b"st" => "sharktorrent",
                b"SZ" => "Shareaza",
                b"TN" => "TorrentDotNET",
                b"TR" => "Transmission",
                b"TS" => "Torrentstorm",
                b"TT" => "TuoTu",
                b"UL" => "uLeecher!",
                b"UT" => "µTorrent",
                b"UW" => "µTorrent Web",
                b"VG" => "Vagal",
                b"WD" => "WebTorrent Desktop",
                b"WT" => "BitLet",
                b"WW" => "WebTorrent",
                b"WY" => "FireTorrent",
                b"XL" => "Xunlei",
                b"XT" => "XanTorrent",
                b"XX" => "Xtorrent",
                b"ZT" => "ZipTorrent",
                _ => return None,
            };
            Some(name)
        } else {
            None
        }
    }
}

impl Serialize for PeerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let buff_size = self.0.len() * 2;
        let mut tmp: Vec<u8> = vec![0; buff_size];
        binascii::bin2hex(&self.0, &mut tmp).unwrap();
        let id = std::str::from_utf8(&tmp).ok();

        #[derive(Serialize)]
        struct PeerIdInfo<'a> {
            id: Option<&'a str>,
            client: Option<&'a str>,
        }

        let obj = PeerIdInfo {
            id,
            client: self.get_client_name(),
        };
        obj.serialize(serializer)
    }
}
