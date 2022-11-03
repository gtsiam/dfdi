use dfdi_core::{Context, Provider, Service};

pub struct CachedRef<'cx, S: Service>(pub S::Output<'cx>);

impl<'cx, S: Service> CachedRef<'cx, S> {
    #[inline(always)]
    pub fn new(value: S::Output<'cx>) -> Self {
        Self(value)
    }
}

impl<'cx, S: Service> Provider<'cx, &'static S> for CachedRef<'cx, S> {
    #[inline(always)]
    fn provide(&'cx self, _cx: &'cx Context) -> &'cx <S as Service>::Output<'cx> {
        &self.0
    }
}
