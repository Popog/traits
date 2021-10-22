//! This crate provides traits which describe functionality of cryptographic hash
//! functions and Message Authentication algorithms.
//!
//! Traits in this repository are organized into the following levels:
//!
//! - **High-level convenience traits**: [`Digest`], [`DynDigest`], [`Mac`].
//!   Wrappers around lower-level traits for most common use-cases.
//! - **Mid-level traits**: [`Update`], [`FixedOutput`], [`ExtendableOutput`],
//!   [`VariableOutput`], [`Reset`], [`XofReader`]. These traits atomically
//!   describe available functionality of an algorithm.
//! - **Marker traits**: [`HashMarker`], [`MacMarker`]. Used to distinguish
//!   different algorithm classes.
//! - **Low-level traits** defined in the [`core_api`] module. These traits
//!   operate at a block-level and do not contain any built-in buffering.
//!   They are intended to be implemented by low-level algorithm providers only
//!   and simplify the amount of work implementers need to do and therefore
//!   usually shouldn't be used in application-level code.
//!
//! Additionally hash functions implement traits from the standard library:
//! [`Default`], [`Clone`], [`Write`]. The latter is
//! feature-gated behind `std` feature, which is usually enabled by default
//! by hash implementation crates.
//!
//! [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/RustCrypto/media/8f1a9894/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/RustCrypto/media/8f1a9894/logo.svg"
)]
#![warn(missing_docs, rust_2018_idioms)]

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

#[cfg(feature = "dev")]
#[cfg_attr(docsrs, doc(cfg(feature = "dev")))]
pub mod dev;

pub mod core_api;
mod digest;
#[cfg(feature = "mac")]
mod mac;

use core::fmt;

pub use crypto_common;
#[cfg(feature = "mac")]
pub use crypto_common::{InnerInit, InvalidLength, Key, KeyInit};
pub use crypto_common::{Output, OutputSizeUser, Reset};
pub use digest::{Digest, DynDigest, HashMarker, InvalidBufferLength};
pub use generic_array::{self, typenum::consts};
#[cfg(feature = "mac")]
pub use mac::{CtOutput, Mac, MacError, MacMarker};

/// Types which consume data with byte granularity.
pub trait Update {
    /// Update state using the provided data.
    fn update(&mut self, data: &[u8]);
}

/// Types which return fixed-sized result after finalization.
pub trait FixedOutput: OutputSizeUser + Sized {
    /// Consume value and write result into provided array.
    fn finalize_into(self, out: &mut Output<Self>);

    /// Retrieve result and consume the hasher instance.
    #[inline]
    fn finalize_fixed(self) -> Output<Self> {
        let mut out = Default::default();
        self.finalize_into(&mut out);
        out
    }
}

/// Types which return fixed-sized result after finalization and reset
/// values into its initial state.
pub trait FixedOutputReset: FixedOutput + Reset {
    /// Write result into provided array and reset value to its initial state.
    fn finalize_into_reset(&mut self, out: &mut Output<Self>);

    /// Retrieve result and reset the hasher instance.
    #[inline]
    fn finalize_fixed_reset(&mut self) -> Output<Self> {
        let mut out = Default::default();
        self.finalize_into_reset(&mut out);
        out
    }
}

/// Trait for describing readers which are used to extract extendable output
/// from XOF (extendable-output function) result.
pub trait XofReader {
    /// Read output into the `buffer`. Can be called an unlimited number of times.
    fn read(&mut self, buffer: &mut [u8]);

    /// Read output into a boxed slice of the specified size.
    ///
    /// Can be called an unlimited number of times in combination with `read`.
    ///
    /// `Box<[u8]>` is used instead of `Vec<u8>` to save stack space, since
    /// they have size of 2 and 3 words respectively.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn read_boxed(&mut self, n: usize) -> Box<[u8]> {
        let mut buf = vec![0u8; n].into_boxed_slice();
        self.read(&mut buf);
        buf
    }
}

/// Trait which describes extendable-output functions (XOF).
pub trait ExtendableOutput: Sized + Update + Reset {
    /// Reader
    type Reader: XofReader;

    /// Retrieve XOF reader and consume hasher instance.
    fn finalize_xof(self) -> Self::Reader;

    /// Retrieve XOF reader and reset hasher instance state.
    fn finalize_xof_reset(&mut self) -> Self::Reader;

    /// Compute hash of `data` and write it to `output`.
    fn digest_xof(input: impl AsRef<[u8]>, output: &mut [u8])
    where
        Self: Default,
    {
        let mut hasher = Self::default();
        hasher.update(input.as_ref());
        hasher.finalize_xof().read(output);
    }

    /// Retrieve result into a boxed slice of the specified size and consume
    /// the hasher.
    ///
    /// `Box<[u8]>` is used instead of `Vec<u8>` to save stack space, since
    /// they have size of 2 and 3 words respectively.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn finalize_boxed(self, output_size: usize) -> Box<[u8]> {
        let mut buf = vec![0u8; output_size].into_boxed_slice();
        self.finalize_xof().read(&mut buf);
        buf
    }

    /// Retrieve result into a boxed slice of the specified size and reset
    /// the hasher's state.
    ///
    /// `Box<[u8]>` is used instead of `Vec<u8>` to save stack space, since
    /// they have size of 2 and 3 words respectively.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn finalize_boxed_reset(&mut self, output_size: usize) -> Box<[u8]> {
        let mut buf = vec![0u8; output_size].into_boxed_slice();
        self.finalize_xof_reset().read(&mut buf);
        buf
    }
}

/// Trait for variable output size hash functions.
pub trait VariableOutput: Sized + Update + Reset {
    /// Maximum size of output hash.
    const MAX_OUTPUT_SIZE: usize;

    /// Create new hasher instance with the given output size.
    ///
    /// It will return `Err(InvalidOutputSize)` in case if hasher can not return
    /// hash of the specified output size.
    fn new(output_size: usize) -> Result<Self, InvalidOutputSize>;

    /// Get output size of the hasher instance provided to the `new` method
    fn output_size(&self) -> usize;

    /// Retrieve result via closure and consume hasher.
    ///
    /// Closure is guaranteed to be called, length of the buffer passed to it
    /// will be equal to `output_size`.
    fn finalize_variable(self, f: impl FnOnce(&[u8]));

    /// Retrieve result via closure and reset the hasher state.
    ///
    /// Closure is guaranteed to be called, length of the buffer passed to it
    /// will be equal to `output_size`.
    fn finalize_variable_reset(&mut self, f: impl FnOnce(&[u8]));

    /// Compute hash of `data` and write it to `output`.
    ///
    /// Length of the output hash is determined by `output`. If `output` is
    /// bigger than `Self::MAX_OUTPUT_SIZE`, this method returns
    /// `InvalidOutputSize`.
    fn digest_variable(
        input: impl AsRef<[u8]>,
        output: &mut [u8],
    ) -> Result<(), InvalidOutputSize> {
        let mut hasher = Self::new(output.len())?;
        hasher.update(input.as_ref());
        hasher.finalize_variable(|out| output.copy_from_slice(out));
        Ok(())
    }

    /// Retrieve result into a boxed slice and consume hasher.
    ///
    /// `Box<[u8]>` is used instead of `Vec<u8>` to save stack space, since
    /// they have size of 2 and 3 words respectively.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn finalize_boxed(self) -> Box<[u8]> {
        let n = self.output_size();
        let mut buf = vec![0u8; n].into_boxed_slice();
        self.finalize_variable(|res| buf.copy_from_slice(res));
        buf
    }

    /// Retrieve result into a boxed slice and reset hasher state.
    ///
    /// `Box<[u8]>` is used instead of `Vec<u8>` to save stack space, since
    /// they have size of 2 and 3 words respectively.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn finalize_boxed_reset(&mut self) -> Box<[u8]> {
        let n = self.output_size();
        let mut buf = vec![0u8; n].into_boxed_slice();
        self.finalize_variable_reset(|res| buf.copy_from_slice(res));
        buf
    }
}

/// The error type for variable hasher initialization.
#[derive(Clone, Copy, Debug, Default)]
pub struct InvalidOutputSize;

impl fmt::Display for InvalidOutputSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid output size")
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl std::error::Error for InvalidOutputSize {}
