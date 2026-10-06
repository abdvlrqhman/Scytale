//! Password and passphrase generator. Every choice is uniform (rejection sampling, no modulo bias).

use rand_core::CryptoRng;
use zeroize::Zeroizing;

use crate::{Error, Result};

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
/// No quotes, backslash or space: they break too many sign-up forms and shells.
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?/~";
const AMBIGUOUS: &str = "Il1O0o";

/// EFF large wordlist (7,776 words), CC BY 3.0 US, Electronic Frontier Foundation.
/// https://www.eff.org/dice — SHA-256 of the original file:
/// addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e
const WORDLIST: &str = include_str!("eff_large_wordlist.txt");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordOptions {
    pub length: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: bool,
    pub avoid_ambiguous: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self { length: 20, lower: true, upper: true, digits: true, symbols: true, avoid_ambiguous: false }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassphraseOptions {
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    /// Appends one random digit to one random word.
    pub include_number: bool,
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        Self { words: 5, separator: "-".into(), capitalize: true, include_number: true }
    }
}

/// Uniform in `0..n`.
fn uniform(rng: &mut impl CryptoRng, n: usize) -> usize {
    let n = u32::try_from(n).expect("small ranges only");
    assert!(n > 0);
    let zone = u32::MAX - (u32::MAX - n + 1) % n; // largest multiple of n, minus 1
    loop {
        let v = rng.next_u32();
        if v <= zone {
            return (v % n) as usize;
        }
    }
}

fn classes(opts: &PasswordOptions) -> Vec<Vec<char>> {
    [(opts.lower, LOWER), (opts.upper, UPPER), (opts.digits, DIGITS), (opts.symbols, SYMBOLS)]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, set)| set.chars().filter(|c| !(opts.avoid_ambiguous && AMBIGUOUS.contains(*c))).collect())
        .collect()
}

/// A random password containing at least one character from every enabled class.
pub fn password(opts: &PasswordOptions, rng: &mut impl CryptoRng) -> Result<Zeroizing<String>> {
    let classes = classes(opts);
    if classes.is_empty() {
        return Err(Error::Invalid("enable at least one character type".into()));
    }
    if !(classes.len().max(4)..=128).contains(&opts.length) {
        return Err(Error::Invalid("length must be between 4 and 128".into()));
    }
    let all: Vec<char> = classes.concat();
    // Rejection sampling keeps the result uniform over all valid passwords.
    loop {
        let pw: Zeroizing<String> = Zeroizing::new((0..opts.length).map(|_| all[uniform(rng, all.len())]).collect());
        if classes.iter().all(|set| pw.chars().any(|c| set.contains(&c))) {
            return Ok(pw);
        }
    }
}

/// Approximate entropy of [`password`] output, for the strength indicator.
pub fn password_entropy_bits(opts: &PasswordOptions) -> f64 {
    let n: usize = classes(opts).iter().map(Vec::len).sum();
    if n == 0 { 0.0 } else { opts.length as f64 * (n as f64).log2() }
}

pub fn passphrase(opts: &PassphraseOptions, rng: &mut impl CryptoRng) -> Result<Zeroizing<String>> {
    if !(3..=20).contains(&opts.words) {
        return Err(Error::Invalid("use between 3 and 20 words".into()));
    }
    let list: Vec<&str> = WORDLIST.lines().collect();
    let number_at = opts.include_number.then(|| uniform(rng, opts.words));
    let mut out = Zeroizing::new(String::new());
    for i in 0..opts.words {
        if i > 0 {
            out.push_str(&opts.separator);
        }
        let word = list[uniform(rng, list.len())];
        if opts.capitalize {
            let mut chars = word.chars();
            out.extend(chars.next().map(|c| c.to_ascii_uppercase()));
            out.push_str(chars.as_str());
        } else {
            out.push_str(word);
        }
        if number_at == Some(i) {
            out.push(char::from(b'0' + uniform(rng, 10) as u8));
        }
    }
    Ok(out)
}

pub fn passphrase_entropy_bits(opts: &PassphraseOptions) -> f64 {
    let words = opts.words as f64 * 7776f64.log2();
    let number = if opts.include_number { (opts.words as f64).log2() + 10f64.log2() } else { 0.0 };
    words + number
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    #[test]
    fn wordlist_is_intact() {
        let words: Vec<&str> = WORDLIST.lines().collect();
        assert_eq!(words.len(), 7776);
        assert_eq!((words[0], words[7775]), ("abacus", "zoom"));
        let unique: std::collections::BTreeSet<_> = words.iter().collect();
        assert_eq!(unique.len(), 7776);
    }

    #[test]
    fn password_respects_options() {
        let rng = &mut ChaCha20Rng::seed_from_u64(1);
        for _ in 0..200 {
            let pw = password(&PasswordOptions::default(), rng).unwrap();
            assert_eq!(pw.len(), 20);
            assert!(pw.chars().any(|c| c.is_ascii_lowercase()));
            assert!(pw.chars().any(|c| c.is_ascii_uppercase()));
            assert!(pw.chars().any(|c| c.is_ascii_digit()));
            assert!(pw.chars().any(|c| SYMBOLS.contains(c)));
        }
        let opts = PasswordOptions { length: 64, symbols: false, avoid_ambiguous: true, ..Default::default() };
        let pw = password(&opts, rng).unwrap();
        assert!(pw.chars().all(|c| c.is_ascii_alphanumeric() && !AMBIGUOUS.contains(c)));
    }

    #[test]
    fn password_rejects_bad_options() {
        let rng = &mut ChaCha20Rng::seed_from_u64(1);
        let none = PasswordOptions { lower: false, upper: false, digits: false, symbols: false, ..Default::default() };
        assert!(password(&none, rng).is_err());
        assert!(password(&PasswordOptions { length: 3, ..Default::default() }, rng).is_err());
        assert!(password(&PasswordOptions { length: 129, ..Default::default() }, rng).is_err());
    }

    #[test]
    fn passphrase_shape() {
        let rng = &mut ChaCha20Rng::seed_from_u64(2);
        let p = passphrase(&PassphraseOptions::default(), rng).unwrap();
        assert!(p.split('-').filter(|w| !w.is_empty()).count() >= 5); // some EFF words contain '-'
        assert_eq!(p.chars().filter(char::is_ascii_digit).count(), 1);
        let plain = PassphraseOptions { words: 4, separator: " ".into(), capitalize: false, include_number: false };
        let p = passphrase(&plain, rng).unwrap();
        assert_eq!(p.split(' ').count(), 4);
        assert!(p.chars().all(|c| c.is_ascii_lowercase() || c == ' ' || c == '-'));
        assert!(passphrase_entropy_bits(&PassphraseOptions::default()) > 64.0);
    }

    #[test]
    fn uniform_is_unbiased_enough() {
        let rng = &mut ChaCha20Rng::seed_from_u64(3);
        let mut counts = [0u32; 7];
        for _ in 0..70_000 {
            counts[uniform(rng, 7)] += 1;
        }
        assert!(counts.iter().all(|&c| (9_400..10_600).contains(&c)), "{counts:?}");
    }
}
