//! Miscellaneous utilities.
use std::{mem, slice, time};

/// Cast a slice to a slice of bytes.
///
/// Result will be native endianness.
///
/// # Safety
///
/// There should be no undefined behaviour with the cast.
pub(crate) unsafe fn as_u8<T>(buf: &[T]) -> &[u8] {
    let ptr = buf.as_ptr() as *const u8;
    let size = buf.len() * mem::size_of::<T>();
    // SAFETY: The required invariants should be met
    //         because we're working from a valid &[T].
    //         - Pointer is not null and will point to valid data.
    //         - Length arithmetic should be good.
    //         - Allocation size restrictions would have been applied
    //           to the slice.
    slice::from_raw_parts(ptr, size)
}

/// Utility for measuring frame rate per second.
///
/// It takes periodic snapshots of the measured
/// fps to slow down the value being printed.
/// This makes it easier to read when presented to
/// the user.
pub struct FpsCounter {
    dt: [f32; 60 * 1],
    snapshot: f32,
    cursor: usize,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            dt: [0.0; 60 * 1],
            snapshot: 0.0,
            cursor: 0,
        }
    }

    pub fn add(&mut self, delta_time: time::Duration) {
        self.dt[self.cursor] = delta_time.as_secs_f32();
        if self.cursor == 0 {
            self.take_snapshot();
        }
        self.cursor = (self.cursor + 1) % self.dt.len();
    }

    fn take_snapshot(&mut self) {
        let sum: f32 = self.dt.iter().fold(0.0, |acc, el| acc + *el);
        let avg = sum / self.dt.len() as f32;
        // Approximately not zero
        if avg.abs() > f32::EPSILON {
            self.snapshot = 1.0 / avg;
        }
    }

    pub fn fps(&self) -> f32 {
        self.snapshot
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_as_u8() {
        todo!()
    }
}
