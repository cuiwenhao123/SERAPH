//! Fixture crate used by `s3-extract` end-to-end tests.
//!
//! ```
//! use s3_audit_fixture::ExampleType;
//!
//! let value = ExampleType::new();
//! assert_eq!(value.value(), 7);
//! ```
//!
//! ```
//! use s3_audit_fixture::SAMPLE_CONST;
//!
//! assert_eq!(SAMPLE_CONST, 7);
//! ```

/// Simple public type for fixture docs.
pub struct ExampleType;

/// Tiny public error type used by constructor-shape tests.
pub struct ExampleError;

/// Public repr-carrying type used to verify layout fact extraction.
#[repr(transparent)]
pub struct ExampleTransparent(pub usize);

/// Public type with an explicit Drop impl used to verify drop fact extraction.
pub struct ExampleDrop;

/// Public type with a manual `Send` impl used to verify unsafe-impl extraction.
pub struct ExampleSend;

impl ExampleType {
    /// Constructs the fixture type.
    pub fn new() -> Self {
        Self
    }

    /// Constructs the fixture type while spelling out the concrete return type.
    pub fn named() -> ExampleType {
        ExampleType
    }

    /// Constructs the fixture type through a thin result wrapper.
    pub fn wrapped() -> Result<Self, ExampleError> {
        Ok(Self)
    }

    /// Returns the fixed fixture value.
    pub fn value(&self) -> usize {
        7
    }
}

impl Drop for ExampleDrop {
    fn drop(&mut self) {}
}

unsafe impl Send for ExampleSend {}

/// Public trait used to verify trait-method extraction.
pub trait ExampleTrait {
    /// Required method example.
    ///
    /// ```
    /// use s3_audit_fixture::{ExampleTrait, ExampleType};
    ///
    /// let value = ExampleType::new();
    /// ExampleTrait::required(&value);
    /// ```
    fn required(&self);

    /// Provided method example.
    ///
    /// ```
    /// use s3_audit_fixture::{ExampleTrait, ExampleType};
    ///
    /// let value = ExampleType::new();
    /// assert_eq!(ExampleTrait::provided(&value), 7);
    /// ```
    fn provided(&self) -> usize {
        7
    }
}

impl ExampleTrait for ExampleType {
    fn required(&self) {}
}

/// Public API using a trait object to verify trait reverse-edge extraction.
pub fn use_trait_object(value: &dyn ExampleTrait) -> usize {
    ExampleTrait::provided(value)
}

/// Public API containing an internal unsafe block while keeping a safe header.
pub fn uses_unsafe_block(ptr: *const u8) -> Option<u8> {
    unsafe { ptr.as_ref().copied() }
}

/// Public extern API used to verify FFI risk-fact extraction.
pub extern "C" fn fixture_ffi_add(left: i32, right: i32) -> i32 {
    left + right
}

/// Public macro used to verify macro extraction.
///
/// ```
/// let value = s3_audit_fixture::fixture_macro!(3);
/// assert_eq!(value, 3);
/// ```
#[macro_export]
macro_rules! fixture_macro {
    ($value:expr) => {
        $value
    };
}

/// Public constant used to verify constant extraction.
///
/// ```
/// assert_eq!(s3_audit_fixture::SAMPLE_CONST, 7);
/// ```
pub const SAMPLE_CONST: usize = 7;
