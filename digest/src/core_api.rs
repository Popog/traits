//! Low-level traits operating on blocks and wrappers around them.
//!
//! Usage of traits in this module in user code is discouraged. Instead use
//! core algorithm wrapped by the wrapper types, which implement the
//! higher-level traits.
use crate::InvalidOutputSize;
use generic_array::ArrayLength;

pub use crypto_common::{AlgorithmName, Block, BlockSizeUser, OutputSizeUser, Reset};

use block_buffer::DigestBuffer;
use crypto_common::Output;

mod ct_variable;
mod rt_variable;
mod wrapper;
mod xof_reader;

pub use ct_variable::CtVariableCoreWrapper;
pub use rt_variable::RtVariableCoreWrapper;
pub use wrapper::CoreWrapper;
pub use xof_reader::XofReaderCoreWrapper;

/// Types which consume data in blocks.
pub trait UpdateCore: BlockSizeUser {
    /// Update state using the provided data blocks.
    fn update_blocks(&mut self, blocks: &[Block<Self>]);
}

/// Types which use [`DigestBuffer`] functionality.
pub trait BufferUser: BlockSizeUser {
    /// Block buffer type over which value operates.
    type Buffer: DigestBuffer<Self::BlockSize>;
}

/// Core trait for hash functions with fixed output size.
pub trait FixedOutputCore: UpdateCore + BufferUser + OutputSizeUser {
    /// Finalize state using remaining data stored in the provided block buffer,
    /// write result into provided array and leave `self` in a dirty state.
    fn finalize_fixed_core(&mut self, buffer: &mut Self::Buffer, out: &mut Output<Self>);
}

/// Core trait for hash functions with extendable (XOF) output size.
pub trait ExtendableOutputCore: UpdateCore + BufferUser {
    /// XOF reader core state.
    type ReaderCore: XofReaderCore;

    /// Retrieve XOF reader using remaining data stored in the block buffer
    /// and leave hasher in a dirty state.
    fn finalize_xof_core(&mut self, buffer: &mut Self::Buffer) -> Self::ReaderCore;
}

/// Core reader trait for extendable-output function (XOF) result.
pub trait XofReaderCore: BlockSizeUser {
    /// Read next XOF block.
    fn read_block(&mut self) -> Block<Self>;
}

/// Core trait for hash functions with variable output size.
pub trait VariableOutputCore: UpdateCore + BufferUser + Sized {
    /// Maximum output size.
    type MaxOutputSize: ArrayLength<u8>;

    /// Initialize hasher state for given output size.
    ///
    /// Returns [`InvalidOutputSize`] if `output_size` is equal to zero or
    /// bigger than `Self::MaxOutputSize`.
    fn new(output_size: usize) -> Result<Self, InvalidOutputSize>;

    /// Finalize hasher and return result of lenght `output_size` via closure `f`.
    ///
    /// `output_size` must be equal to `output_size` used during construction.
    fn finalize_variable_core(
        &mut self,
        buffer: &mut Self::Buffer,
        output_size: usize,
        f: impl FnOnce(&[u8]),
    );
}
