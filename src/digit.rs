use std::mem;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Digit {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
}

use Digit::*;

impl Digit {
    pub fn carrying_add(self, other: Digit, carry: bool) -> (Digit, bool) {
        let s: usize = self.into();
        let o: usize = other.into();
        let sum = s + o + usize::from(carry);
        if sum > 9 {
            (unsafe { Digit::new_unchecked(sum - 10) }, true)
        } else {
            (unsafe { Digit::new_unchecked(sum) }, false)
        }
    }

    pub fn carrying_mul(self, other: Digit, carry: Digit) -> (Digit, Digit) {
        let s: usize = self.into();
        let o: usize = other.into();
        let c: usize = carry.into();
        let prod = s * o + c;
        let (tens, ones) = (prod / 10, prod % 10);
        (
            unsafe { Digit::new_unchecked(ones) },
            unsafe { Digit::new_unchecked(tens) },
        )
    }

    pub fn carrying_sub(self, other: Digit) -> (Digit, bool) {
        let s: usize = self.into();
        let o: usize = other.into();
        if s < o {
            (unsafe { Digit::new_unchecked(s + 10 - o) }, true)
        } else {
            (unsafe { Digit::new_unchecked(s - o) }, false)
        }
    }

    pub fn increment(&mut self) {
        match self {
            Zero => *self = One,
            One => *self = Two,
            Two => *self = Three,
            Three => *self = Four,
            Four => *self = Five,
            Five => *self = Six,
            Six => *self = Seven,
            Seven => *self = Eight,
            Eight => *self = Nine,
            Nine => panic!("cannot increment `Nine`"),
        }
    }

    pub fn wrapping_decrement(&mut self) -> bool {
        match self {
            Zero => *self = Nine,
            One => *self = Zero,
            Two => *self = One,
            Three => *self = Two,
            Four => *self = Three,
            Five => *self = Four,
            Six => *self = Five,
            Seven => *self = Six,
            Eight => *self = Seven,
            Nine => *self = Eight,
        }
        self == &Nine
    }

    pub unsafe fn new_unchecked(digit: usize) -> Digit {
        mem::transmute(digit as u8)
    }
}

impl From<Digit> for usize {
    fn from(digit: Digit) -> usize {
        match digit {
            Zero => 0,
            One => 1,
            Two => 2,
            Three => 3,
            Four => 4,
            Five => 5,
            Six => 6,
            Seven => 7,
            Eight => 8,
            Nine => 9,
        }
    }
}

impl From<Digit> for char {
    fn from(digit: Digit) -> char {
        match digit {
            Zero => '0',
            One => '1',
            Two => '2',
            Three => '3',
            Four => '4',
            Five => '5',
            Six => '6',
            Seven => '7',
            Eight => '8',
            Nine => '9',
        }
    }
}

impl TryFrom<usize> for Digit {
    type Error = ();
    fn try_from(i: usize) -> Result<Digit, ()> {
        match i {
            0 => Ok(Zero),
            1 => Ok(One),
            2 => Ok(Two),
            3 => Ok(Three),
            4 => Ok(Four),
            5 => Ok(Five),
            6 => Ok(Six),
            7 => Ok(Seven),
            8 => Ok(Eight),
            9 => Ok(Nine),
            _ => Err(()),
        }
    }
}

impl TryFrom<char> for Digit {
    type Error = ();
    fn try_from(c: char) -> Result<Digit, ()> {
        match c {
            '0' => Ok(Zero),
            '1' => Ok(One),
            '2' => Ok(Two),
            '3' => Ok(Three),
            '4' => Ok(Four),
            '5' => Ok(Five),
            '6' => Ok(Six),
            '7' => Ok(Seven),
            '8' => Ok(Eight),
            '9' => Ok(Nine),
            _ => Err(()),
        }
    }
}
