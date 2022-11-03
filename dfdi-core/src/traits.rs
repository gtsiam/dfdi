use crate::Context;

/// A can construct a [`Service`] which references objects either inside itself or the provided
/// [`Context`].
///
/// Any type implementing the appropriate [`Fn`] trait can be used as a provider:
/// ```
/// # use dfdi::{Service, Context};
/// # use std::convert::Infallible;
/// // A service providing a random number
/// #[derive(Service)]
/// struct Random(u64);
///
/// // Create a context and bind Random to a provider
/// let mut cx = Context::new();
/// cx.bind_fn::<Random>(|_cx: &Context| Random(rand::random()));
///
/// // Print a random number
/// println!("{}", cx.resolve::<Random>().0);
/// ```
pub trait Provider<'cx, S: Service>: Send + Sync + 'cx {
    /// Build the output object
    // #! Remember to keep in sync with `ProvideFn`
    fn provide(&'cx self, cx: &'cx Context, arg: S::Argument<'_>) -> S::Output<'cx>;
}

/// A pointer to the underlying provider function.
///
/// The first argument must be a pointer, otherwise miri's stacked borrows will reject this code.
/// This is because casting a `*const ()` to, say, a `&'cx ()` resizes the alloc range that the
/// reference has access to down to zero bytes, causing Miri to report undefined behaviour.
///
/// As such, it is up to the caller to ensure that the first argument lives for a lifetime of 'cx.
///
/// # SAFETY
/// - The first argument must have the correct type
/// - The first argument must live for 'cx
// #! This __MUST__ be kept in sync with `Provider::provide` or bad things will happen
pub(crate) type ProvideFn<'cx, S> =
    unsafe fn(*const (), &'cx Context, <S as Service>::Argument<'_>) -> <S as Service>::Output<'cx>;

/// A key to an object that can be created by a [`Provider`] and stored in a [`Context`].
///
/// In most cases, an implementation of this trait is trivial boilerplate, and so it is recommended
/// to use the provided derive macro.
pub trait Service: 'static {
    /// The result of a service resolution
    type Output<'cx>;

    type Argument<'arg>;
}
