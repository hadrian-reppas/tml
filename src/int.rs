use std::cmp::Ordering;
use std::ops::{Add, Mul, Sub};
use std::str::FromStr;
use std::{fmt, iter};

use crate::decimal::Decimal;
use crate::digit::Digit::{self, *};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Int(Vec<Digit>);

impl Int {
    pub fn zero() -> Int {
        Int(Vec::new())
    }

    pub fn one() -> Int {
        Int(vec![One])
    }

    pub fn from_hex_digit(c: char) -> Int {
        match c {
            '0'..='9' => Int::from(c as usize - '0' as usize),
            'a'..='f' => Int::from(c as usize - 'a' as usize + 10),
            'A'..='F' => Int::from(c as usize - 'A' as usize + 10),
            _ => panic!("not a hex digit"),
        }
    }

    pub fn pow(&self, mut exp: u64) -> Int {
        if exp == 0 {
            return Int::one();
        }

        let mut base = self.clone();
        let mut acc = Int::one();
        while exp > 1 {
            if (exp & 1) == 1 {
                acc = &acc * &base;
            }
            exp /= 2;
            base = &base * &base;
        }

        &acc * &base
    }

    pub fn times_ten(mut self) -> Int {
        self.0.insert(0, Zero);
        self
    }

    pub fn divmod(&self, denom: &Int) -> (Digit, Int) {
        let mut digit = Zero;
        let mut prod = Int::zero();
        while digit < Nine {
            let next = &prod + denom;
            if &next > self {
                break;
            }
            digit.increment();
            prod = next;
        }
        let rem = self - &prod;
        (digit, rem)
    }

    pub fn inverse(&self, len: usize) -> Decimal {
        assert!(!self.0.is_empty(), "divide by zero");

        let mut digits = Vec::new();
        let mut minuend = Int::from(10);
        for _ in 0..len {
            let (digit, rem) = minuend.divmod(self);
            digits.push(digit);
            minuend = rem.times_ten();
        }
        digits.reverse();
        Decimal::new(digits)
    }

    pub fn digits(&self) -> &[Digit] {
        &self.0
    }
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "0")
        } else {
            for &digit in self.0.iter().rev() {
                write!(f, "{}", char::from(digit))?;
            }
            Ok(())
        }
    }
}

impl From<usize> for Int {
    fn from(mut i: usize) -> Int {
        let mut int = Vec::new();
        while i != 0 {
            int.push(Digit::try_from(i % 10).unwrap());
            i /= 10;
        }
        Int(int)
    }
}

impl Add for &Int {
    type Output = Int;
    fn add(self, rhs: &Int) -> Int {
        if self.0.len() < rhs.0.len() {
            add_impl(&rhs.0, &self.0)
        } else {
            add_impl(&self.0, &rhs.0)
        }
    }
}

fn add_impl(big: &[Digit], small: &[Digit]) -> Int {
    let mut sum = Vec::with_capacity(big.len() + 1);
    let mut digit;
    let mut carry = false;
    for (&a, &b) in big.iter().zip(small.iter().chain(iter::repeat(&Zero))) {
        (digit, carry) = a.carrying_add(b, carry);
        sum.push(digit);
    }
    if carry {
        sum.push(One);
    }
    Int(sum)
}

impl Sub for &Int {
    type Output = Int;
    fn sub(self, rhs: &Int) -> Int {
        assert!(self >= rhs, "overflow when subtracting `Int`s");

        let mut digits = self.0.clone();
        let mut diff = Vec::new();
        for (mut i, &b) in rhs
            .0
            .iter()
            .chain(iter::repeat(&Zero))
            .enumerate()
            .take(digits.len())
        {
            let (digit, carry) = digits[i].carrying_sub(b);
            diff.push(digit);
            if carry {
                i += 1;
                while digits[i].wrapping_decrement() {
                    i += 1;
                }
            }
        }

        while diff.last() == Some(&Zero) {
            diff.pop();
        }
        Int(diff)
    }
}

impl Mul for &Int {
    type Output = Int;
    fn mul(self, rhs: &Int) -> Int {
        let mut prod = Int::zero();
        for (i, &a) in self.0.iter().enumerate() {
            let mut term = vec![Zero; i];
            let mut carry = Zero;
            let mut digit;
            for &b in &rhs.0 {
                (digit, carry) = a.carrying_mul(b, carry);
                term.push(digit);
            }
            if carry != Zero {
                term.push(carry);
            }
            if term.iter().any(|&d| d != Zero) {
                prod = &prod + &Int(term);
            }
        }
        prod
    }
}

impl PartialOrd for Int {
    fn partial_cmp(&self, rhs: &Int) -> Option<Ordering> {
        let cmp = self.0.len().cmp(&rhs.0.len());
        if cmp != Ordering::Equal {
            return Some(cmp);
        }

        for (a, b) in self.0.iter().zip(rhs.0.iter()).rev() {
            if a.cmp(b) != Ordering::Equal {
                return Some(a.cmp(b));
            }
        }

        Some(Ordering::Equal)
    }
}

impl Ord for Int {
    fn cmp(&self, rhs: &Int) -> Ordering {
        self.partial_cmp(rhs).unwrap()
    }
}

impl FromStr for Int {
    type Err = ();
    fn from_str(s: &str) -> Result<Int, ()> {
        let mut digits = Vec::with_capacity(s.len());
        for c in s.chars().rev() {
            digits.push(Digit::try_from(c)?);
        }
        Ok(Int(digits))
    }
}
