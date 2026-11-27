use crate::Uuid;
use crate::timestamp::UnixTimestamp;

mod private {
    pub trait Sealed {}
}

pub trait UuidRfcVersion: private::Sealed {}

pub struct Version1 {
    pub(crate) something: String,
}

impl private::Sealed for Version1 {}
impl UuidRfcVersion for Version1 {}

pub struct Version4 {
    pub(crate) something: u32,
}

impl private::Sealed for Version4 {}
impl UuidRfcVersion for Version4 {}

pub struct Version7 {
    pub(crate) strict: bool,
    pub(crate) additional_precision: bool,
    pub(crate) crypto: bool,
    pub(crate) timestamp: UnixTimestamp,
    pub(crate) global: bool,
}

impl private::Sealed for Version7 {}
impl UuidRfcVersion for Version7 {}

pub struct UuidBuilder<T>(T)
where
    T: UuidRfcVersion;

impl UuidBuilder<Version1> {
    pub fn version1() -> Self {
        Self(Version1 {
            something: String::new(),
        })
    }

    pub fn something(mut self, value: String) -> Self {
        self.0.something = value;
        self
    }
}

impl UuidBuilder<Version4> {
    pub fn version4() -> Self {
        Self(Version4 { something: 0 })
    }

    pub fn something(mut self, value: u32) -> Self {
        self.0.something = value;
        self
    }
}

impl UuidBuilder<Version7> {
    pub fn version7() -> Self {
        Self(Version7 {
            strict: false,
            additional_precision: false,
            crypto: false,
            timestamp: UnixTimestamp::now(),
            global: false,
        })
    }

    /// By default we use the current Unix timestamp, e.g.`UnixTimestamp::now()`
    pub fn timestamp(mut self, unix_timestamp: UnixTimestamp) -> Self {
        self.0.timestamp = unix_timestamp;
        self
    }

    /// The UUID Version 7 RFC only requires a 48 bit timestamp with millisecond monotonicity,
    /// with the option of using additional sub-millisecond precision. This allows for better
    /// sorting and obviously sub-millisecond monotonicity.
    pub fn additional_precision(mut self) -> Self {
        self.0.additional_precision = true;
        self
    }

    /// The UUID Version 7 RFC does not require sub-millisecond monotonicity.
    pub fn strict(mut self) -> Self {
        self.0.strict = true;
        self
    }

    /// Process-wide global monotonicity.
    pub fn global(mut self) -> Self {
        self.0.global = true;
        self
    }

    /// Use cryptographically secure RNG (uses XorShift128 otherwise).
    pub fn crypto(mut self) -> Self {
        self.0.crypto = true;
        self
    }

    pub fn build(self) -> Uuid {
        Uuid::from(self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn builder_foo() {
        let v7 = UuidBuilder::version7()
            .crypto()
            .strict()
            .additional_precision()
            .build();

        let v7 = UuidBuilder::version7()
            .crypto()
            .strict()
            .additional_precision()
            .build();

        // let x = UuidBuilder::version7();
        // let v1 = UuidBuilder::<Version1>::version1().something(String::new());
    }
}
