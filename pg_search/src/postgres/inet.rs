// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::fmt::{Display, Formatter};
use std::net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use thiserror::Error;

const IPV4_FAMILY_MARKER: u8 = 4;
const IPV6_FAMILY_MARKER: u8 = 6;

const MASK_TERMINATOR: u8 = 0b00;
const ZERO_BIT: u8 = 0b01;
const ONE_BIT: u8 = 0b10;
const SYMBOLS_PER_BYTE: usize = 4;

/// A complete PostgreSQL `inet` value.
///
/// PostgreSQL comparison semantics depend on all three components: address family,
/// address bits, and mask length. Keeping them separate prevents IPv4-mapped IPv6
/// addresses and differently masked addresses from collapsing to the same value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InetValue {
    address: IpAddr,
    mask: u8,
}

impl InetValue {
    pub(crate) fn new(address: IpAddr, mask: u8) -> Result<Self, InetValueError> {
        let max_bits = max_bits(address);
        if mask > max_bits {
            return Err(InetValueError::MaskOutOfRange { mask, max_bits });
        }
        Ok(Self { address, mask })
    }

    /// Encode this value so lexicographic byte order matches PostgreSQL's `inet`
    /// comparison order.
    ///
    /// PostgreSQL compares same-family values by:
    /// 1. address bits through the shorter mask;
    /// 2. mask length;
    /// 3. all remaining address bits.
    ///
    /// The encoded symbol stream is:
    ///
    /// ```text
    /// family | network-prefix bits | mask terminator | host bits
    /// ```
    ///
    /// Address bits use ordered two-bit symbols (`01` for zero, `10` for one),
    /// while the mask terminator is `00`. Therefore, when two network prefixes
    /// are equal, a shorter mask sorts before either possible next address bit.
    pub(crate) fn encode(self) -> Vec<u8> {
        let family_marker = match self.address {
            IpAddr::V4(_) => IPV4_FAMILY_MARKER,
            IpAddr::V6(_) => IPV6_FAMILY_MARKER,
        };
        let address_bytes = address_bytes(self.address);
        let addr_bit_count = max_bits(self.address) as usize;

        let mut symbols = Vec::with_capacity(addr_bit_count + 1);
        for bit_index in 0..self.mask as usize {
            symbols.push(encode_address_bit(bit_at(&address_bytes, bit_index)));
        }
        symbols.push(MASK_TERMINATOR);
        for bit_index in self.mask as usize..addr_bit_count {
            symbols.push(encode_address_bit(bit_at(&address_bytes, bit_index)));
        }

        // Pack four 2-bit symbols into each output byte instead of storing every
        // symbol in its own `u8`, reducing the encoded size.
        let mut encoded = Vec::with_capacity(1 + symbols.len().div_ceil(SYMBOLS_PER_BYTE));
        encoded.push(family_marker);

        for chunk in symbols.chunks(SYMBOLS_PER_BYTE) {
            let mut byte = 0;
            for symbol_index in 0..SYMBOLS_PER_BYTE {
                byte <<= 2;
                byte |= chunk.get(symbol_index).copied().unwrap_or(MASK_TERMINATOR);
            }
            encoded.push(byte);
        }

        encoded
    }

    /// Decode an `inet` value produced by `InetValue::encode`
    pub(crate) fn decode(encoded: &[u8]) -> Result<Self, InetValueError> {
        let Some((&family_marker, packed_symbols)) = encoded.split_first() else {
            return Err(InetValueError::EmptyEncoding);
        };

        let (addr_bit_count, addr_byte_count) = match family_marker {
            IPV4_FAMILY_MARKER => (Ipv4Addr::BITS as usize, 4),
            IPV6_FAMILY_MARKER => (Ipv6Addr::BITS as usize, 16),
            marker => return Err(InetValueError::InvalidFamilyMarker(marker)),
        };
        let symbol_count = addr_bit_count + 1;
        let expected_packed_len = symbol_count.div_ceil(SYMBOLS_PER_BYTE);
        if packed_symbols.len() != expected_packed_len {
            return Err(InetValueError::InvalidEncodingLength {
                actual: encoded.len(),
                expected: expected_packed_len + 1,
            });
        }

        let mut address = vec![0_u8; addr_byte_count];
        let mut mask = 0;
        let mut found_mask_terminator = false;
        let mut address_bit_index = 0;

        for symbol_index in 0..symbol_count {
            let symbol = symbol_at(packed_symbols, symbol_index);
            match symbol {
                MASK_TERMINATOR if !found_mask_terminator => {
                    mask = address_bit_index as u8;
                    found_mask_terminator = true;
                }
                ZERO_BIT | ONE_BIT => {
                    if address_bit_index >= addr_bit_count {
                        return Err(InetValueError::MissingMaskTerminator);
                    }
                    set_bit(&mut address, address_bit_index, u8::from(symbol == ONE_BIT));
                    address_bit_index += 1;
                }
                MASK_TERMINATOR => return Err(InetValueError::MultipleMaskTerminators),
                invalid => {
                    return Err(InetValueError::InvalidSymbol {
                        symbol: invalid,
                        index: symbol_index,
                    });
                }
            }
        }

        if trailing_padding(packed_symbols, symbol_count) != 0 {
            return Err(InetValueError::NonZeroPadding);
        }

        let address = match family_marker {
            IPV4_FAMILY_MARKER => {
                let octets: [u8; 4] = address
                    .try_into()
                    .expect("IPv4 address length was validated");
                IpAddr::V4(Ipv4Addr::from(octets))
            }
            IPV6_FAMILY_MARKER => {
                let octets: [u8; 16] = address
                    .try_into()
                    .expect("IPv6 address length was validated");
                IpAddr::V6(Ipv6Addr::from(octets))
            }
            _ => unreachable!("family marker was validated"),
        };

        Self::new(address, mask)
    }
}

impl FromStr for InetValue {
    type Err = InetValueError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        let (address_text, mask_text) = match input.split_once('/') {
            Some((address, mask)) => (address, Some(mask)),
            None => (input, None),
        };
        let address = address_text.parse::<IpAddr>()?;
        let mask = match mask_text {
            Some(mask) => mask
                .parse::<u8>()
                .map_err(|_| InetValueError::InvalidMask(mask.to_owned()))?,
            None => max_bits(address),
        };

        Self::new(address, mask)
    }
}

impl Display for InetValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.address)?;
        if self.mask != max_bits(self.address) {
            write!(formatter, "/{}", self.mask)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub(crate) enum InetValueError {
    #[error("invalid IP address: {0}")]
    InvalidAddress(#[from] AddrParseError),

    #[error("invalid inet mask: {0}")]
    InvalidMask(String),

    #[error("inet mask {mask} exceeds address width {max_bits}")]
    MaskOutOfRange { mask: u8, max_bits: u8 },

    #[error("inet encoding is empty")]
    EmptyEncoding,

    #[error("invalid inet family marker: {0}")]
    InvalidFamilyMarker(u8),

    #[error("invalid inet encoding length {actual}; expected {expected}")]
    InvalidEncodingLength { actual: usize, expected: usize },

    #[error("invalid two-bit symbol {symbol:#04b} at index {index}")]
    InvalidSymbol { symbol: u8, index: usize },

    #[error("inet encoding has no mask terminator")]
    MissingMaskTerminator,

    #[error("inet encoding has multiple mask terminators")]
    MultipleMaskTerminators,

    #[error("inet encoding has non-zero padding")]
    NonZeroPadding,
}

fn max_bits(address: IpAddr) -> u8 {
    match address {
        IpAddr::V4(_) => Ipv4Addr::BITS as u8,
        IpAddr::V6(_) => Ipv6Addr::BITS as u8,
    }
}

fn address_bytes(address: IpAddr) -> Vec<u8> {
    match address {
        IpAddr::V4(address) => address.octets().to_vec(),
        IpAddr::V6(address) => address.octets().to_vec(),
    }
}

fn encode_address_bit(bit: u8) -> u8 {
    match bit {
        0 => ZERO_BIT,
        1 => ONE_BIT,
        _ => unreachable!("address bits are always zero or one"),
    }
}

/// Return the bit at `bit_index` as 0 or 1.
/// `bit_index / 8` selects its byte; shifting moves that bit to the right edge,
/// and `& 1` removes every other bit.
fn bit_at(bytes: &[u8], bit_index: usize) -> u8 {
    (bytes[bit_index / 8] >> (7 - bit_index % 8)) & 1
}

/// Add the decoded bit to its position in the zero-filled address bytes.
/// Shifting places the bit correctly; OR preserves bits written previously.
fn set_bit(bytes: &mut [u8], bit_index: usize, bit: u8) {
    bytes[bit_index / 8] |= bit << (7 - bit_index % 8);
}

/// Extract the symbol at `symbol_index` from the packed byte array.
/// Select its byte, shift its two bits to the right edge, then mask off other bits.
///
/// Returns a 2 bit symbol.
fn symbol_at(packed_symbols: &[u8], symbol_index: usize) -> u8 {
    let byte = packed_symbols[symbol_index / SYMBOLS_PER_BYTE];
    let shift = 6 - 2 * (symbol_index % SYMBOLS_PER_BYTE);
    (byte >> shift) & 0b11
}

fn trailing_padding(packed_symbols: &[u8], symbol_count: usize) -> u8 {
    let used_symbols_in_last_byte = symbol_count % SYMBOLS_PER_BYTE;
    if used_symbols_in_last_byte == 0 {
        return 0;
    }

    let padding_bits = 2 * (SYMBOLS_PER_BYTE - used_symbols_in_last_byte);
    packed_symbols.last().copied().unwrap_or_default() & ((1 << padding_bits) - 1)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use proptest::prelude::*;

    use super::*;

    #[test]
    fn parses_default_and_explicit_masks() {
        let ipv4: InetValue = "192.168.1.5".parse().unwrap();
        assert_eq!(ipv4.address, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)));
        assert_eq!(ipv4.mask, 32);

        let ipv4_network: InetValue = "192.168.1.5/24".parse().unwrap();
        assert_eq!(ipv4_network.address, ipv4.address);
        assert_eq!(ipv4_network.mask, 24);

        let ipv6: InetValue = "2001:db8::1".parse().unwrap();
        assert!(matches!(ipv6.address, IpAddr::V6(_)));
        assert_eq!(ipv6.mask, 128);
    }

    #[test]
    fn rejects_invalid_values_and_masks() {
        assert!(matches!(
            "not-an-address".parse::<InetValue>(),
            Err(InetValueError::InvalidAddress(_))
        ));
        assert!(matches!(
            "192.168.1.5/not-a-mask".parse::<InetValue>(),
            Err(InetValueError::InvalidMask(_))
        ));
        assert!(matches!(
            "192.168.1.5/33".parse::<InetValue>(),
            Err(InetValueError::MaskOutOfRange {
                mask: 33,
                max_bits: 32
            })
        ));
        assert!(matches!(
            "::1/129".parse::<InetValue>(),
            Err(InetValueError::MaskOutOfRange {
                mask: 129,
                max_bits: 128
            })
        ));
    }

    #[test]
    fn encoding_is_compact_and_reversible() {
        for input in [
            "0.0.0.0/0",
            "192.168.1.5/24",
            "192.168.1.5",
            "::/0",
            "2001:db8::1/64",
            "::ffff:1.2.3.4",
        ] {
            let value: InetValue = input.parse().unwrap();
            let encoded = value.encode();
            assert_eq!(encoded.len(), if value.address.is_ipv4() { 10 } else { 34 });
            assert_eq!(InetValue::decode(&encoded).unwrap(), value);
        }
    }

    #[test]
    fn mask_is_part_of_equality() {
        let network: InetValue = "192.168.1.5/24".parse().unwrap();
        let host: InetValue = "192.168.1.5/32".parse().unwrap();

        assert_ne!(network, host);
        assert_ne!(network.encode(), host.encode());
    }

    #[test]
    fn every_ipv4_value_sorts_before_every_ipv6_value() {
        let last_ipv4: InetValue = "255.255.255.255".parse().unwrap();
        let first_ipv6: InetValue = "::/0".parse().unwrap();
        let mapped_ipv6: InetValue = "::ffff:1.2.3.4".parse().unwrap();

        assert!(last_ipv4.encode() < first_ipv6.encode());
        assert!(last_ipv4.encode() < mapped_ipv6.encode());
    }

    #[test]
    fn ipv4_and_mapped_ipv6_do_not_collapse() {
        let ipv4: InetValue = "1.2.3.4".parse().unwrap();
        let mapped_ipv6: InetValue = "::ffff:1.2.3.4".parse().unwrap();

        assert_ne!(ipv4, mapped_ipv6);
        assert_ne!(ipv4.encode(), mapped_ipv6.encode());
    }

    #[test]
    fn shorter_equal_prefix_sorts_before_longer_mask() {
        let shorter_mask: InetValue = "10.0.0.1/8".parse().unwrap();
        let longer_mask: InetValue = "10.0.0.0/16".parse().unwrap();

        assert_eq!(postgres_cmp(&shorter_mask, &longer_mask), Ordering::Less);
        assert!(shorter_mask.encode() < longer_mask.encode());
    }

    #[test]
    fn rejects_malformed_encodings() {
        assert!(matches!(
            InetValue::decode(&[]),
            Err(InetValueError::EmptyEncoding)
        ));
        assert!(matches!(
            InetValue::decode(&[5]),
            Err(InetValueError::InvalidFamilyMarker(5))
        ));

        let valid: InetValue = "192.168.1.5/24".parse().unwrap();
        let encoded = valid.encode();
        assert!(matches!(
            InetValue::decode(&encoded[..encoded.len() - 1]),
            Err(InetValueError::InvalidEncodingLength { .. })
        ));

        let mut invalid_symbol = encoded.clone();
        invalid_symbol[1] = 0b11_00_00_00;
        assert!(matches!(
            InetValue::decode(&invalid_symbol),
            Err(InetValueError::InvalidSymbol {
                symbol: 0b11,
                index: 0
            })
        ));

        let mut missing_terminator = vec![IPV4_FAMILY_MARKER];
        missing_terminator.extend([0b01_01_01_01; 8]);
        missing_terminator.push(0b01_00_00_00);
        assert!(matches!(
            InetValue::decode(&missing_terminator),
            Err(InetValueError::MissingMaskTerminator)
        ));

        let mut non_zero_padding = encoded;
        *non_zero_padding.last_mut().unwrap() |= 1;
        assert!(matches!(
            InetValue::decode(&non_zero_padding),
            Err(InetValueError::NonZeroPadding)
        ));
    }

    proptest! {
        #[test]
        fn ipv4_roundtrips_and_preserves_postgres_order(
            lhs_octets in any::<[u8; 4]>(),
            lhs_mask in 0_u8..=32,
            rhs_octets in any::<[u8; 4]>(),
            rhs_mask in 0_u8..=32,
        ) {
            let lhs = InetValue::new(IpAddr::V4(Ipv4Addr::from(lhs_octets)), lhs_mask).unwrap();
            let rhs = InetValue::new(IpAddr::V4(Ipv4Addr::from(rhs_octets)), rhs_mask).unwrap();

            prop_assert_eq!(InetValue::decode(&lhs.encode()).unwrap(), lhs);
            prop_assert_eq!(
                lhs.encode().cmp(&rhs.encode()),
                postgres_cmp(&lhs, &rhs)
            );
        }

        #[test]
        fn ipv6_roundtrips_and_preserves_postgres_order(
            lhs_octets in any::<[u8; 16]>(),
            lhs_mask in 0_u8..=128,
            rhs_octets in any::<[u8; 16]>(),
            rhs_mask in 0_u8..=128,
        ) {
            let lhs = InetValue::new(IpAddr::V6(Ipv6Addr::from(lhs_octets)), lhs_mask).unwrap();
            let rhs = InetValue::new(IpAddr::V6(Ipv6Addr::from(rhs_octets)), rhs_mask).unwrap();

            prop_assert_eq!(InetValue::decode(&lhs.encode()).unwrap(), lhs);
            prop_assert_eq!(
                lhs.encode().cmp(&rhs.encode()),
                postgres_cmp(&lhs, &rhs)
            );
        }
    }

    // Test-only reference implementation of PostgreSQL `inet` ordering, used to
    // verify that lexicographical comparison of encoded bytes gives the same result.
    fn postgres_cmp(lhs: &InetValue, rhs: &InetValue) -> Ordering {
        match (lhs.address, rhs.address) {
            (IpAddr::V4(_), IpAddr::V6(_)) => return Ordering::Less,
            (IpAddr::V6(_), IpAddr::V4(_)) => return Ordering::Greater,
            _ => {}
        }

        let lhs_bytes = address_bytes(lhs.address);
        let rhs_bytes = address_bytes(rhs.address);
        let shared_prefix_bits = usize::from(lhs.mask.min(rhs.mask));

        for bit_index in 0..shared_prefix_bits {
            match bit_at(&lhs_bytes, bit_index).cmp(&bit_at(&rhs_bytes, bit_index)) {
                Ordering::Equal => {}
                ordering => return ordering,
            }
        }

        match lhs.mask.cmp(&rhs.mask) {
            Ordering::Equal => {}
            ordering => return ordering,
        }

        lhs_bytes.cmp(&rhs_bytes)
    }
}
