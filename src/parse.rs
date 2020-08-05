use crate::ByteUnit;

macro_rules! parse_suffix_fn {
    ($($suffix:ident),*) => (
        parse_suffix_fn!($($suffix, stringify!($suffix)),*);
    );
    ($($suffix:ident, $string:expr),*) => (
        fn parse_suffix(string: &str) -> Option<ByteUnit> {
            $(if string.eq_ignore_ascii_case($string) {
                return Some(ByteUnit::$suffix);
            })*

            None
        }
    );
}

parse_suffix_fn!(B, kB, KiB, MB, MiB, GB, GiB, TB, TiB, PB, PiB, EB, EiB);

fn is_suffix_char(c: char) -> bool {
    "begikmpt ".contains(c.to_ascii_lowercase())
}

impl core::str::FromStr for ByteUnit {
    type Err = Option<(usize, char)>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() { return Err(None); }
        let (mut dot, mut suffix) = (None, None);
        for (i, c) in s.chars().enumerate() {
            match c {
                c if c.is_ascii_digit() && suffix.is_none() => continue,
                '.' if dot.is_none() && suffix.is_none() => dot = Some(i),
                c if is_suffix_char(c) && suffix.is_none() => suffix = Some(i),
                c if is_suffix_char(c) => continue,
                _ => Err((i, c))?
            }
        }

        // We can't start with `.` or a suffix character.
        if dot.map(|i| i == 0).unwrap_or(false) || suffix.map(|i| i == 0).unwrap_or(false) {
            return Err(None);
        }

        // Parse the suffix. A fractional doesn't make sense for bytes.
        let suffix_str = suffix.map(|i| s[i..].trim_start()).unwrap_or("b");
        let unit = parse_suffix(suffix_str).ok_or(None)?;
        if unit == ByteUnit::B && dot.is_some() {
            return Err(dot.map(|i| (i, s.as_bytes()[i] as char)));
        }

        let num_end = suffix.unwrap_or(s.len());
        match dot {
            Some(i) => {
                let frac_str = &s[(i + 1)..num_end];
                let whole: u64 = s[..i].parse().or(Err(None))?;
                let frac: u32 = frac_str.parse().or(Err(None))?;
                let frac_part = frac as f64 / 10u64.pow(frac_str.len() as u32) as f64;
                let frac_unit = (frac_part * unit.as_u64() as f64) as u64;
                Ok(whole * unit + frac_unit)
            }
            None => {
                let whole: u64 = s[..num_end].parse().or(Err(None))?;
                Ok(whole * unit)
            }
        }
    }
}

#[cfg(test)]
mod parse_tests {
    use core::str::FromStr;
    use crate::{ByteUnit, ToByteUnit};

    macro_rules! assert_reject {
        ($($s:expr),* $(,)?) => ($(
            let result = ByteUnit::from_str($s);
            assert!(result.is_err(), "{:?} parsed as {}", $s, result.unwrap());
        )*)
    }

    macro_rules! assert_parses {
        ($($s:expr => $b:expr),* $(,)?) => ($(
            let result = ByteUnit::from_str($s);
            assert!(result.is_ok(), "{:?} failed to parse", $s);
            let actual = result.unwrap();
            assert_eq!(actual, $b, "expected {}, got {}", $b, actual);
        )*)
    }

    #[test]
    fn reject() {
        assert_reject!["", "a", "amb", "1.2", ".", ".23KiB", "1.2.3mb", "1.23bcc"];
        assert_reject!["?mb", "1.2mb.", ".2mb", "99k", "99bk", "1 k b"];
        assert_reject!["1.2mkb", "1kb2", "1MB ", " 1MB"];
        assert_reject!["287423890740938348498349344"];
        assert_reject!["1.2mkb", "1kb2", "1MB ", " 1MB"];
    }

    #[test]
    fn accept() {
        assert_parses! {
            "1" => 1.bytes(),
            "123" => 123.bytes(),
            "99" => 99.bytes(),
            "2394394" => 2394394.bytes(),
            "2874238907409384" => 2874238907409384u64.bytes(),
        }

        assert_parses! {
            "1 b" => 1.bytes(),
            "1 B" => 1.bytes(),
            "1B" => 1.bytes(),
            "1b" => 1.bytes(),
            "1 mb" => 1.megabytes(),
            "1 mib" => 1.mebibytes(),
            "1 kb" => 1.kilobytes(),
            "1 kB" => 1.kilobytes(),
            "1 kib" => 1.kibibytes(),
            "1kib" => 1.kibibytes(),
            "1KiB" => 1.kibibytes(),
            "1GB" => 1.gigabytes(),

            "349 b" => 349.bytes(),
            "13489 mb" => 13489.megabytes(),
            "349b" => 349.bytes(),
            "13489mb" => 13489.megabytes(),
        }

        assert_parses! {
            "0.5KiB" => 512.bytes(),
            "0.5KB" => 500.bytes(),
        }

        assert_parses! {
            "323kB" => 323.kilobytes(),
            "3MB" => 3.megabytes(),
            "3MiB" => 3.mebibytes(),
            "8GiB" => 8.gibibytes(),

            "5MiB" => 3.mebibytes() + 2.mebibytes(),
            "7.06GB" => 7.gigabytes() + 60.megabytes(),
            "7.25GB" => 7.gigabytes() + 250.megabytes(),

            "01MB" => 1.megabytes(),
            "0001MiB" => 1.mebibytes(),
        }
    }
}
