use crate::UUID_BYTES;
use crate::Uuid;
use crate::timestamp::UnixTimestamp;

pub struct Version1 {
    pub(crate) something: String,
}

pub struct Version4 {
    pub(crate) something: u32,
}

pub struct Version7 {
    // Ensures all **thread-local** IDs are strictly monotonic.
    pub(crate) strict: bool,
    pub(crate) additional_precision: bool,
    pub(crate) crypto: bool,
    pub(crate) timestamp: UnixTimestamp,
    pub(crate) global: bool,
}

pub struct UuidBuilder<T>(T);

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
