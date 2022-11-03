//! Utilities for building [`Providers`](dfdi_core::Provider)

mod cached;
mod cached_ref;

pub use cached::Cached;
pub use cached_ref::CachedRef;

use dfdi_core::{Context, Provider, Service};

/// Type hint to the rust compiler to treat appropriately typed closures as providers.
///
/// This may become unnecessary once type inference improves a bit, but for now it's useful to have.
#[inline(always)]
pub fn provider_fn<'cx, S: Service>(
    func: impl Fn(&'cx Context) -> S::Output<'cx> + 'cx,
) -> impl Provider<'cx, S> {
    func
}
