use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Custom wrapper so we don't have to write serialization/deserialization code manually
#[derive(Serialize, Deserialize)]
struct KZGCommitment(#[serde(with = "hex::serde")] pub(super) [u8; 48]);

impl Serialize for super::KZGCommitment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        KZGCommitment(self.to_bytes()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for super::KZGCommitment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let KZGCommitment(bytes) = KZGCommitment::deserialize(deserializer)?;
        Self::try_from_bytes(&bytes).map_err(|error| D::Error::custom(format!("{error:?}")))
    }
}

// Custom wrapper so we don't have to write serialization/deserialization code manually
#[derive(Serialize, Deserialize)]
struct KZGProof(#[serde(with = "hex::serde")] pub(super) [u8; 48]);

impl Serialize for super::KZGProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        KZGProof(self.to_bytes()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for super::KZGProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let KZGProof(bytes) = KZGProof::deserialize(deserializer)?;
        Self::try_from_bytes(&bytes).map_err(|error| D::Error::custom(format!("{error:?}")))
    }
}

// #[derive(Serialize, Deserialize)]
// struct KZGCommitmentWrapper(#[serde(with = "hex::serde")] pub [u8; BYTES_PER_G1]);

// #[derive(Serialize, Deserialize)]
// struct KZGProofWrapper(#[serde(with = "hex::serde")] pub [u8; BYTES_PER_G1]);

// #[derive(Serialize, Deserialize)]
// struct BlsScalarWrapper(#[serde(with = "hex::serde")] pub [u8; BYTES_PER_FIELD_ELEMENT]);