use std::fmt;
use std::ops::{Add, Mul};
use std::str::FromStr;

use crate::digit::Digit::{self, *};
use crate::int::Int;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal(Vec<Digit>);

impl Decimal {
    pub fn new(digits: Vec<Digit>) -> Decimal {
        Decimal(digits)
    }

    pub fn zero(len: usize) -> Decimal {
        Decimal(vec![Zero; len])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn digits(&self) -> &[Digit] {
        &self.0
    }

    pub fn trim(mut self, n: usize) -> Decimal {
        self.0.drain(..n);
        self
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0.")?;
        for &digit in self.0.iter().rev() {
            write!(f, "{}", char::from(digit))?;
        }
        Ok(())
    }
}

impl Add for &Decimal {
    type Output = Decimal;
    fn add(self, rhs: &Decimal) -> Decimal {
        assert_eq!(self.len(), rhs.len());

        let mut sum = Vec::with_capacity(self.len());
        let mut digit;
        let mut carry = false;
        for (&a, &b) in self.0.iter().zip(rhs.0.iter()) {
            (digit, carry) = a.carrying_add(b, carry);
            sum.push(digit);
        }
        assert!(!carry, "overflow when adding `Decimal`s");

        Decimal(sum)
    }
}

impl Mul<&Int> for &Decimal {
    type Output = Decimal;
    fn mul(self, rhs: &Int) -> Decimal {
        let mut prod = Decimal::zero(self.len());
        for (i, &a) in rhs.digits().iter().enumerate() {
            let mut term = vec![Zero; i];
            let mut carry = Zero;
            let mut digit;
            for &b in &self.0 {
                (digit, carry) = a.carrying_mul(b, carry);
                term.push(digit);
            }
            term.push(carry);
            while term.len() > self.len() {
                assert_eq!(
                    term.pop().unwrap(),
                    Zero,
                    "overflow when multiplying `Decimal`s"
                );
            }
            prod = &prod + &Decimal(term);
        }
        prod
    }
}

impl Mul<&Decimal> for &Int {
    type Output = Decimal;
    fn mul(self, rhs: &Decimal) -> Decimal {
        rhs * self
    }
}

impl FromStr for Decimal {
    type Err = ();
    fn from_str(s: &str) -> Result<Decimal, ()> {
        let mut digits = Vec::with_capacity(s.len().saturating_sub(2));
        let suffix = s.strip_prefix("0.").ok_or(())?;
        for c in suffix.chars().rev() {
            digits.push(Digit::try_from(c)?);
        }
        Ok(Decimal(digits))
    }
}
