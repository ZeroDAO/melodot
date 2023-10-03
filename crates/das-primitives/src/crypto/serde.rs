use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::BYTES_PER_G1;

macro_rules! kzg_type_wrapper {
	($name:ident, $size:expr) => {
		#[derive(Serialize, Deserialize)]
		struct $name(#[serde(with = "hex::serde")] pub(super) [u8; $size]);

		impl Serialize for super::$name {
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				$name(self.to_bytes()).serialize(serializer)
			}
		}

		impl<'de> Deserialize<'de> for super::$name {
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				let $name(bytes) = $name::deserialize(deserializer)?;
				Self::try_from_bytes(&bytes).map_err(|error| D::Error::custom(format!("{error:?}")))
			}
		}
	};
}

kzg_type_wrapper!(KZGCommitment, BYTES_PER_G1);
kzg_type_wrapper!(KZGProof, BYTES_PER_G1);
