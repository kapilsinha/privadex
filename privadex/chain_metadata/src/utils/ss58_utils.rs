pub use ss58_registry::{from_known_address_format, Ss58AddressFormat, Ss58AddressFormatRegistry};

use crate::common::{PublicError, Result};


const PREFIX: &[u8] = b"SS58PRE";
pub fn ss58hash(data: &[u8]) -> blake2_rfc::blake2b::Blake2bResult {
	let mut context = blake2_rfc::blake2b::Blake2b::new(64);
	context.update(PREFIX);
	context.update(data);
	context.finalize()
}

// Re-export underlying codec if std is allowed
#[cfg(feature = "std")]
pub use sp_core::crypto::Ss58Codec;

#[cfg(not(feature = "std"))]
use base58::{FromBase58, ToBase58};
#[cfg(not(feature = "std"))]
use ink_prelude::{
    string::String,
    vec,
};

/// Key that can be encoded to/from SS58.
///
/// See <https://docs.substrate.io/v3/advanced/ss58/>
/// for information on the codec.
#[cfg(not(feature = "std"))]
pub trait Ss58Codec: Sized + AsMut<[u8]> + AsRef<[u8]> + sp_core::crypto::ByteArray {
    /// A format filterer, can be used to ensure that `from_ss58check` family only decode for
	/// allowed identifiers. By default just refuses the two reserved identifiers.
	fn format_is_allowed(f: Ss58AddressFormat) -> bool {
		!f.is_reserved()
	}

	/// Some if the string is a properly encoded SS58Check address.
	fn from_ss58check(s: &str) -> Result<Self> {
		Self::from_ss58check_with_version(s).and_then(|(r, v)| match v {
			v if !v.is_custom() => Ok(r),
			v if v == Self::default_ss58_version() => Ok(r),
			v => Err(PublicError::UnknownSs58AddressFormat(v)),
		})
	}

	/// Some if the string is a properly encoded SS58Check address.
	fn from_ss58check_with_version(s: &str) -> Result<(Self, Ss58AddressFormat)> {
		const CHECKSUM_LEN: usize = 2;
		let body_len = Self::LEN;

		let data = s.from_base58().map_err(|_| PublicError::BadBase58)?;
		if data.len() < 2 {
			return Err(PublicError::BadLength)
		}
		let (prefix_len, ident) = match data[0] {
			0..=63 => (1, data[0] as u16),
			64..=127 => {
				// weird bit manipulation owing to the combination of LE encoding and missing two
				// bits from the left.
				// d[0] d[1] are: 01aaaaaa bbcccccc
				// they make the LE-encoded 16-bit value: aaaaaabb 00cccccc
				// so the lower byte is formed of aaaaaabb and the higher byte is 00cccccc
				let lower = (data[0] << 2) | (data[1] >> 6);
				let upper = data[1] & 0b00111111;
				(2, (lower as u16) | ((upper as u16) << 8))
			},
			_ => return Err(PublicError::InvalidPrefix),
		};
		if data.len() != prefix_len + body_len + CHECKSUM_LEN {
			return Err(PublicError::BadLength)
		}
		let format = ident.into();
		if !Self::format_is_allowed(format) {
			return Err(PublicError::FormatNotAllowed)
		}

		let hash = ss58hash(&data[0..body_len + prefix_len]);
		let checksum = &hash.as_bytes()[0..CHECKSUM_LEN];
		if data[body_len + prefix_len..body_len + prefix_len + CHECKSUM_LEN] != *checksum {
			// Invalid checksum.
			return Err(PublicError::InvalidChecksum)
		}

		let result = Self::from_slice(&data[prefix_len..body_len + prefix_len])
			.map_err(|()| PublicError::BadLength)?;
		Ok((result, format))
	}

	/// Some if the string is a properly encoded SS58Check address, optionally with
	/// a derivation path following.
	fn from_string(s: &str) -> Result<Self> {
		Self::from_string_with_version(s).and_then(|(r, v)| match v {
			v if !v.is_custom() => Ok(r),
			v if v == Self::default_ss58_version() => Ok(r),
			v => Err(PublicError::UnknownSs58AddressFormat(v)),
		})
	}

	/// Return the ss58-check string for this key.
	fn to_ss58check_with_version(&self, version: Ss58AddressFormat) -> String {
		// We mask out the upper two bits of the ident - SS58 Prefix currently only supports 14-bits
		let ident: u16 = u16::from(version) & 0b0011_1111_1111_1111;
		let mut v = match ident {
			0..=63 => vec![ident as u8],
			64..=16_383 => {
				// upper six bits of the lower byte(!)
				let first = ((ident & 0b0000_0000_1111_1100) as u8) >> 2;
				// lower two bits of the lower byte in the high pos,
				// lower bits of the upper byte in the low pos
				let second = ((ident >> 8) as u8) | ((ident & 0b0000_0000_0000_0011) as u8) << 6;
				vec![first | 0b01000000, second]
			},
			_ => unreachable!("masked out the upper two bits; qed"),
		};
		v.extend(self.as_ref());
		let r = ss58hash(&v);
		v.extend(&r.as_bytes()[0..2]);
		v.to_base58()
	}

	/// Return the ss58-check string for this key.
	fn to_ss58check(&self) -> String {
		self.to_ss58check_with_version(Self::default_ss58_version())
	}

	/// Some if the string is a properly encoded SS58Check address, optionally with
	/// a derivation path following.
	fn from_string_with_version(s: &str) -> Result<(Self, Ss58AddressFormat)> {
		Self::from_ss58check_with_version(s)
	}

	/// Returns default SS58 format used by the current active process.
	fn default_ss58_version() -> Ss58AddressFormat {
		Ss58AddressFormat::custom(from_known_address_format(Ss58AddressFormatRegistry::SubstrateAccount))
	}
}


#[cfg(not(feature = "std"))]
impl Ss58Codec for sp_core::crypto::AccountId32 {}
