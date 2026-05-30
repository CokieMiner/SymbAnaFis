use alloc::format;
use core::fmt;

use super::super::traits::Number;
use super::CliffordNumber;
use super::types::blade_label;

impl fmt::Display for CliffordNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let limit = self.blade_count();
        let mut first = true;

        for blade in 0..limit {
            let coeff = &self.coeffs_slice()[blade];
            if coeff.is_zero() {
                continue;
            }
            let basis = blade_label(self.generator_set(), blade);
            let s = format!("{coeff}");
            let (is_neg, mag) = s
                .strip_prefix('-')
                .map_or((false, s.as_str()), |rest| (true, rest));

            if first {
                if is_neg {
                    f.write_str("-")?;
                }
                if mag != "1" || basis.is_empty() {
                    f.write_str(mag)?;
                }
                f.write_str(&basis)?;
                first = false;
            } else {
                f.write_str(if is_neg { " - " } else { " + " })?;
                if mag != "1" || basis.is_empty() {
                    f.write_str(mag)?;
                }
                f.write_str(&basis)?;
            }
        }
        if first {
            f.write_str("0")?;
        }
        Ok(())
    }
}
