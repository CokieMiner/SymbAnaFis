//! Literal rendering without negating fixed-width minimum components.

use core::fmt::{Display, Formatter, Result as FmtResult};

use alloc::string::ToString;

use super::NumericValue;

impl NumericValue {
    /// Render a negative real coefficient's magnitude without native negation.
    pub fn format_magnitude(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let text = self.to_string();
        formatter.write_str(text.strip_prefix('-').unwrap_or(&text))
    }
}

impl Display for NumericValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        if !self.0.is_complex() {
            return Display::fmt(&self.0, formatter);
        }
        let Some(parts) = self.parts() else {
            return Display::fmt(&self.0, formatter);
        };
        if parts.re.is_zero() {
            if parts.im.is_one() {
                formatter.write_str("i")
            } else if parts.im.is_neg_one() {
                formatter.write_str("-i")
            } else {
                write!(formatter, "{}*i", parts.im)
            }
        } else {
            write!(formatter, "{}", parts.re)?;
            if parts.im.is_negative() {
                formatter.write_str(" - ")?;
                if !parts.im.is_neg_one() {
                    let text = parts.im.to_string();
                    formatter.write_str(text.strip_prefix('-').unwrap_or(&text))?;
                    formatter.write_str("*")?;
                }
            } else {
                formatter.write_str(" + ")?;
                if !parts.im.is_one() {
                    write!(formatter, "{}*", parts.im)?;
                }
            }
            formatter.write_str("i")
        }
    }
}
