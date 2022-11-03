use crate::{Context, Provider, Service};

/// Allow `Fn` functions to act as providers.
impl<'cx, F, S> Provider<'cx, S> for F
where
    F: Fn(&'cx Context, S::Argument<'_>) -> S::Output<'cx> + Send + Sync + 'cx,
    S: Service,
{
    fn provide(&'cx self, cx: &'cx Context, arg: S::Argument<'_>) -> S::Output<'cx> {
        (self)(cx, arg)
    }
}

impl<S: Service> Service for &'static S {
    type Output<'cx> = &'cx S::Output<'cx>;
    type Argument<'arg> = S::Argument<'arg>;
}

impl<S: Service> Service for &'static mut S {
    type Output<'cx> = &'cx mut S::Output<'cx>;
    type Argument<'arg> = S::Argument<'arg>;
}
