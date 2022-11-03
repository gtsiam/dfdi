#![forbid(unsafe_code)]

pub use dfdi_core::{BindError, Context, Provider, Service, UnbindError};

#[cfg(feature = "derive")]
pub use dfdi_macros::Service;

mod cached;
mod cached_service;

pub use cached::Cached;
pub use cached_service::CachedService;

/// Type hint to the rust compiler to treat appropriately typed closures as providers.
///
/// This may become unnecessary once type inference improves a bit, but for now it's useful to have.
#[inline(always)]
pub fn provider_fn<'cx, S: Service>(
    func: impl Fn(&'cx Context, S::Argument<'_>) -> S::Output<'cx> + Send + Sync + 'cx,
) -> impl Provider<'cx, S> {
    func
}
