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
        if self.0.len() == 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_plus_one() {
        let sum = &Int::one() + &Int::one();
        assert_eq!(sum, Int::from(2));
        assert_eq!(sum.to_string(), "2");
    }

    #[test]
    fn zero_plus_one() {
        let sum = &Int::zero() + &Int::one();
        assert_eq!(sum, Int::one());
        assert_eq!(sum.to_string(), "1");
    }

    #[test]
    fn one_plus_zero() {
        let sum = &Int::one() + &Int::zero();
        assert_eq!(sum, Int::one());
        assert_eq!(sum.to_string(), "1");
    }

    #[test]
    fn zero_plus_zero() {
        let sum = &Int::zero() + &Int::zero();
        assert_eq!(sum, Int::zero());
        assert_eq!(sum.to_string(), "0");
    }

    #[test]
    fn one_plus_nine() {
        let sum = &Int::one() + &Int::from(9);
        assert_eq!(sum, Int::from(10));
        assert_eq!(sum.to_string(), "10");
    }

    #[test]
    fn nine_plus_ten() {
        let sum = &Int::from(9) + &Int::from(10);
        assert_eq!(sum, Int::from(19));
        assert_eq!(sum.to_string(), "19");
    }

    #[test]
    fn ten_plus_nine() {
        let sum = &Int::from(10) + &Int::from(9);
        assert_eq!(sum, Int::from(19));
        assert_eq!(sum.to_string(), "19");
    }

    #[test]
    fn big_nums() {
        let sum = &Int::from(1234) + &Int::from(8766);
        assert_eq!(sum, Int::from(10000));
        assert_eq!(sum.to_string(), "10000");
    }

    #[test]
    fn two_times_two() {
        let prod = &Int::from(2) * &Int::from(2);
        assert_eq!(prod, Int::from(4));
        assert_eq!(prod.to_string(), "4");
    }

    #[test]
    fn ten_times_zero() {
        let prod = &Int::from(10) * &Int::zero();
        assert_eq!(prod, Int::zero());
        assert_eq!(prod.to_string(), "0");
    }

    #[test]
    fn ten_times_ten() {
        let prod = &Int::from(10) * &Int::from(10);
        assert_eq!(prod, Int::from(100));
        assert_eq!(prod.to_string(), "100");
    }

    #[test]
    fn nines() {
        let prod = &Int::from(999) * &Int::from(9999);
        assert_eq!(prod, Int::from(9989001));
        assert_eq!(prod.to_string(), "9989001");
    }

    #[test]
    fn ten_minus_ten() {
        let diff = &Int::from(10) - &Int::from(10);
        assert_eq!(diff, Int::zero());
        assert_eq!(diff.to_string(), "0");
    }

    #[test]
    fn ten_minus_zero() {
        let diff = &Int::from(10) - &Int::zero();
        assert_eq!(diff, Int::from(10));
        assert_eq!(diff.to_string(), "10");
    }

    #[test]
    fn ten_minus_nine() {
        let diff = &Int::from(10) - &Int::from(9);
        assert_eq!(diff, Int::one());
        assert_eq!(diff.to_string(), "1");
    }

    #[test]
    fn minus_nines() {
        let diff = &Int::from(200) - &Int::from(99);
        assert_eq!(diff, Int::from(101));
        assert_eq!(diff.to_string(), "101");
    }

    #[test]
    fn zero_pow_zero() {
        let pow = Int::zero().pow(0);
        assert_eq!(pow, Int::from(1));
        assert_eq!(pow.to_string(), "1");
    }

    #[test]
    fn ten_pow_zero() {
        let pow = Int::from(10).pow(0);
        assert_eq!(pow, Int::from(1));
        assert_eq!(pow.to_string(), "1");
    }

    #[test]
    fn two_squared() {
        let pow = Int::from(2).pow(2);
        assert_eq!(pow, Int::from(4));
        assert_eq!(pow.to_string(), "4");
    }

    #[test]
    fn big_pow() {
        let pow = Int::from(16).pow(15);
        assert_eq!(pow, Int::from(1152921504606846976));
        assert_eq!(pow.to_string(), "1152921504606846976");
    }

    #[test]
    fn really_big_pow() {
        let pow = Int::from(569).pow(325);
        assert_eq!(pow.to_string(), "25792092543319015842646883112249406223751957442730105221981757938208848185018466459774393266627172027593729054909000987972800310153774493285632476537974407572792429348565962863201540178290570798346795335370659236038862212895483709515790793365723066045700150191799257604044574629915843930649657296326467481571683700728943297017748758377460183056212550788970103692815959562361429228812816668305330820724564633783996066242746056681170114716016775270574898045113401334483268944461336172949776743195545092272979491042308101947822706736225622197776149924416533417585137724129026978891423860673998403589408852430901607532241778424977067374430927185733485448771409053639210975912175925009502047602160050256072476767686654622606104664427472110573093923669584182783428989809743322148286362397855058991814280450518467242907639812288650944025213339934807835922248032233478714290989029991657897974885100900249");
    }

    #[test]
    fn two_pow_64() {
        let pow = Int::from(2).pow(64);
        let expected: Int = "18446744073709551616".parse().unwrap();
        assert_eq!(pow, expected);
        assert_eq!(pow.to_string(), "18446744073709551616");
    }

    #[test]
    fn hex_digits() {
        assert_eq!(Int::from_hex_digit('0'), Int::zero());
        assert_eq!(Int::from_hex_digit('1'), Int::one());
        assert_eq!(Int::from_hex_digit('a'), Int::from(10));
        assert_eq!(Int::from_hex_digit('A'), Int::from(10));
        assert_eq!(Int::from_hex_digit('f'), Int::from(15));
        assert_eq!(Int::from_hex_digit('F'), Int::from(15));
    }

    #[test]
    fn cmp_one_zero() {
        assert!(Int::zero() < Int::one());
        assert!(Int::one() > Int::zero());
    }

    #[test]
    fn cmp_same_len() {
        assert!(Int::from(10) < Int::from(11));
        assert!(Int::from(11) <= Int::from(11));
        assert!(Int::from(99) > Int::from(98));
    }

    #[test]
    fn cmp() {
        assert!(Int::from(146) < Int::from(270));
    }

    #[test]
    fn one_half() {
        let half = Int::from(2).inverse(5);
        assert_eq!(half.to_string(), "0.50000");
    }

    #[test]
    fn quarter() {
        let quarter = Int::from(4).inverse(5);
        assert_eq!(quarter.to_string(), "0.25000");
        let half = &Int::from(2) * &quarter;
        assert_eq!(half.to_string(), "0.50000");
    }

    #[test]
    fn big() {
        let inverse = Int::from(73).inverse(15);
        assert_eq!(inverse.to_string(), "0.013698630136986");
    }
}
