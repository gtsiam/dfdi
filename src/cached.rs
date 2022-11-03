use once_cell::sync::OnceCell;

use dfdi_core::{Context, Provider, Service};

/// Cached provider
///
/// A provider that calls the underlying provider on the first call and returns the result of that
/// on all calls
pub struct Cached<'cx, S, P>
where
    S: Service,
    P: Provider<'cx, S>,
{
    provider: P,
    cache: OnceCell<S::Output<'cx>>,
}

impl<'cx, S, P> Cached<'cx, S, P>
where
    S: Service,
    P: Provider<'cx, S>,
{
    /// Create a new cached provider
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            cache: OnceCell::new(),
        }
    }
}

impl<'cx, S, F> Cached<'cx, S, F>
where
    S: Service,
    F: Fn(&'cx Context, S::Argument<'_>) -> S::Output<'cx> + Send + Sync + 'cx,
{
    /// Equivelant to calling [`Cached::new`] with a provider wrapped in a
    /// [`provider_fn`](crate::provider_fn) type hint
    #[inline(always)]
    pub fn new_fn(provider: F) -> Self {
        Self::new(provider)
    }
}

impl<'cx, S, P> Provider<'cx, &'static S> for Cached<'cx, S, P>
where
    S: Service,
    S::Output<'cx>: Send + Sync,
    P: Provider<'cx, S>,
{
    fn provide(&'cx self, cx: &'cx Context, arg: S::Argument<'_>) -> &'cx S::Output<'cx> {
        self.cache.get_or_init(|| self.provider.provide(cx, arg))
    }
}

impl<'cx, S, P> Default for Cached<'cx, S, P>
where
    S: Service,
    P: Provider<'cx, S> + Default,
{
    #[inline]
    fn default() -> Self {
        Self::new(P::default())
    }
}
