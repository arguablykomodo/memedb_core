// #region Log Address trait
#[cfg(logAddresses)]
pub mod log_address {
    use std::iter::*;

    pub trait LogAddress<Item, I: Iterator<Item = Item>> {
        fn log<'a>(self) -> Map<Enumerate<I>, &'a Fn((usize, Item)) -> Item>;
    }

    impl<Item, I> LogAddress<Item, I> for I
    where
        I: Iterator<Item = Item>,
    {
        fn log<'a>(self) -> Map<Enumerate<I>, &'a Fn((usize, Item)) -> Item> {
            self.enumerate().map(&|(a, v)| {
                debug!("Address: 0x{:06X?}", a);
                v
            })
        }
    }
}
#[cfg(not(logAddresses))]
pub mod log_address {
    pub trait LogAddress<I: Iterator> {
        fn log(self) -> I;
    }
    impl<I> LogAddress<I> for I
    where
        I: Iterator,
    {
        fn log(self) -> I {
            self
        }
    }
}
// #endregion

macro_rules! next {
    ($i:ident) => {
        $i.next().ok_or(Error::EOF)??
    };
    ($i:ident; peek) => {
        $i.peek().and_then(|v| v.as_ref().ok())
    };
}

macro_rules! open_file {
    ($i:literal, $n:expr) => {
        BufReader::new(File::open($i).unwrap()).bytes().skip($n)
    };
}

macro_rules! tagset {
    {$($tag:expr),*} => {
        {
            let mut m = TagSet::new();
            $(
                m.insert($tag.to_string());
            )*
            m
        }
    };
}
