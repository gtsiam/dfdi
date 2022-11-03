use dfdi_core::{Context, Provider, Service};

/// Cached service
///
/// A provider that returns the same reference on every call
pub struct CachedService<'cx, S: Service>(pub S::Output<'cx>);

impl<'cx, S: Service> CachedService<'cx, S> {
    /// Create a new cached service
    #[inline(always)]
    pub fn new(value: S::Output<'cx>) -> Self {
        Self(value)
    }
}

impl<'cx, S: Service> Provider<'cx, &'static S> for CachedService<'cx, S> {
    #[inline(always)]
    fn provide(&'cx self, _cx: &'cx Context) -> &'cx <S as Service>::Output<'cx> {
        &self.0
    }
}
