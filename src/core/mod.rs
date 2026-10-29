//! Password generation has no dependency on the UI, filesystem or network.

use zeroize::Zeroizing;

pub const MIN_LENGTH: usize = 4;
pub const MAX_RANDOM_LENGTH: usize = 64;
pub const MAX_PIN_LENGTH: usize = 12;

const GROUPS: [&[u8]; 4] = [
    b"abcdefghijklmnopqrstuvwxyz",
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    b"0123456789",
    b"!@#$",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterTypes(u8);

impl CharacterTypes {
    pub const ALL: Self = Self(0b1111);

    pub fn contains(self, index: usize) -> bool {
        index < 4 && self.0 & (1 << index) != 0
    }

    /// Refuse to deselect the final group. Invalid group indexes are ignored.
    pub fn toggle(&mut self, index: usize) -> bool {
        if index >= 4 || self.0 == 1 << index {
            return false;
        }
        self.0 ^= 1 << index;
        true
    }

    pub fn alphabet(self) -> Vec<u8> {
        GROUPS
            .iter()
            .enumerate()
            .filter(|(index, _)| self.contains(*index))
            .flat_map(|(_, chars)| chars.iter().copied())
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerateError {
    InvalidLength,
    EmptyAlphabet,
    RandomUnavailable,
}

pub trait RandomSource {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<(), GenerateError>;
}

pub struct SystemRandom;

impl RandomSource for SystemRandom {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<(), GenerateError> {
        getrandom::fill(bytes).map_err(|_| GenerateError::RandomUnavailable)
    }
}

pub fn generate(
    length: usize,
    pin: bool,
    types: CharacterTypes,
) -> Result<Zeroizing<String>, GenerateError> {
    generate_with(length, pin, types, &mut SystemRandom)
}

pub fn generate_with(
    length: usize,
    pin: bool,
    types: CharacterTypes,
    random: &mut impl RandomSource,
) -> Result<Zeroizing<String>, GenerateError> {
    let maximum = if pin {
        MAX_PIN_LENGTH
    } else {
        MAX_RANDOM_LENGTH
    };
    if !(MIN_LENGTH..=maximum).contains(&length) {
        return Err(GenerateError::InvalidLength);
    }
    let alphabet = if pin {
        GROUPS[2].to_vec()
    } else {
        types.alphabet()
    };
    if alphabet.is_empty() {
        return Err(GenerateError::EmptyAlphabet);
    }

    // Reject the incomplete final bucket instead of introducing modulo bias.
    let limit = (256 / alphabet.len()) * alphabet.len();
    let mut buffer = Zeroizing::new([0_u8; 64]);
    let mut password = Zeroizing::new(String::with_capacity(length));
    while password.len() < length {
        random.fill(buffer.as_mut_slice())?;
        for &byte in buffer.iter() {
            if usize::from(byte) < limit {
                password.push(alphabet[usize::from(byte) % alphabet.len()] as char);
                if password.len() == length {
                    break;
                }
            }
        }
    }
    Ok(password)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sequence(u8);
    impl RandomSource for Sequence {
        fn fill(&mut self, bytes: &mut [u8]) -> Result<(), GenerateError> {
            for byte in bytes {
                *byte = self.0;
                self.0 = self.0.wrapping_add(1);
            }
            Ok(())
        }
    }

    #[test]
    fn every_nonempty_selection_uses_only_its_alphabet() {
        for mask in 1..=15 {
            let types = CharacterTypes(mask);
            let password = generate_with(64, false, types, &mut Sequence(180)).unwrap();
            assert_eq!(password.len(), 64);
            assert!(password.bytes().all(|b| types.alphabet().contains(&b)));
        }
    }

    #[test]
    fn pin_ignores_character_selection_and_preserves_leading_zeroes() {
        let password = generate_with(12, true, CharacterTypes(0), &mut Sequence(0)).unwrap();
        assert_eq!(password.len(), 12);
        assert!(password.starts_with('0'));
        assert!(password.bytes().all(|b| b.is_ascii_digit()));
    }

    #[test]
    fn invalid_lengths_and_empty_alphabet_are_rejected() {
        for (length, pin) in [(0, false), (3, true), (65, false), (13, true)] {
            assert_eq!(
                generate_with(length, pin, CharacterTypes::ALL, &mut Sequence(0)).err(),
                Some(GenerateError::InvalidLength)
            );
        }
        assert_eq!(
            generate_with(20, false, CharacterTypes(0), &mut Sequence(0)).err(),
            Some(GenerateError::EmptyAlphabet)
        );
    }

    #[test]
    fn all_valid_selection_transitions_keep_at_least_one_type() {
        for mask in 1..=15 {
            for index in 0..4 {
                let mut types = CharacterTypes(mask);
                let changed = types.toggle(index);
                assert_ne!(types.0, 0);
                assert_eq!(changed, mask != 1 << index);
            }
        }
        let mut types = CharacterTypes::ALL;
        assert!(!types.toggle(4));
        assert_eq!(types, CharacterTypes::ALL);
    }

    #[test]
    fn random_failure_never_returns_a_fallback_password() {
        struct Failure;
        impl RandomSource for Failure {
            fn fill(&mut self, _: &mut [u8]) -> Result<(), GenerateError> {
                Err(GenerateError::RandomUnavailable)
            }
        }
        assert_eq!(
            generate_with(20, false, CharacterTypes::ALL, &mut Failure).err(),
            Some(GenerateError::RandomUnavailable)
        );
    }

    #[test]
    fn incomplete_buckets_are_rejected() {
        struct RejectFirst(bool);
        impl RandomSource for RejectFirst {
            fn fill(&mut self, bytes: &mut [u8]) -> Result<(), GenerateError> {
                if self.0 {
                    return Err(GenerateError::RandomUnavailable);
                }
                bytes.fill(255);
                self.0 = true;
                Ok(())
            }
        }
        assert_eq!(
            generate_with(20, false, CharacterTypes::ALL, &mut RejectFirst(false)).err(),
            Some(GenerateError::RandomUnavailable)
        );
    }

    #[test]
    fn operating_system_random_source_is_available() {
        assert!(generate(20, false, CharacterTypes::ALL).is_ok());
    }
}
