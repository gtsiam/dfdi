use crate::Context;

/// A `Provider<'cx, S>` is an object that can construct a [`Service`] `S`  which references objects
/// either inside the provided `Context` or the provider itself.
///
/// Any type implementing the appropriate [`FnMut`] trait can be used as a provider:
/// ```
/// # use dfdi::{Service, Context};
/// # use std::convert::Infallible;
/// // A service providing a random number
/// struct Random(u64);
///
/// impl Service for Random {
///     type Error = Infallible;
///     type Output<'cx> = Random;
/// }
///
/// // Create a context and bind Random to a provider
/// let mut cx = Context::new();
/// cx.bind_with::<Random>(|_cx: &Context| Ok(Random(rand::random())));
///
/// // Print a random number
/// println!("{}", cx.resolve::<Random>().unwrap().0);
///
/// ```
///
/// # (!) Closures returning references
/// Rust has trouble inferring the required lifetime bounds for closures when they return
/// non-`'static` services. Instead, you can use an explicit function, or hint at the compiler what
/// the correct lifetimes are using either [`util::provider_fn`](crate::util::provider_fn) or
/// [`Context::bind_fn`](crate::Context::bind_fn).
///
/// ```
/// # use dfdi::{Service, Provider, Context, ProviderError};
/// # use std::convert::Infallible;
///
/// #[derive(Service)]
/// struct Random(u64);
///
/// // Create a context and bind Random to a provider
/// let mut cx = Context::new();
/// cx.bind_with::<Random>(|_cx: &Context| Ok(Random(rand::random())));
///
/// ```
pub trait Provider<'cx, S: Service>: 'cx {
    /// Build the output object
    // #! Remember to keep in synx with `ProvideFn`
    fn provide(&'cx self, cx: &'cx Context) -> S::Output<'cx>;
}

/// A pointer to the underlying provider function.
///
/// The first argument must be a *const () pointer, otherwise miri's stacked borrows will reject
/// this code. This is because casting a `*const ()` to, say, a `&'cx ()` resizes the alloc range
/// that the reference has access to down to zero bytes. At which point; it all falls apart.
///
/// As such, it is up to the caller to ensure that the first argument lives for a lifetime of 'cx.
///
/// # SAFETY
/// - The first argument must have the correct type
/// - The first argument must live for 'cx
// #! This __MUST__ be kept in sync with `Provider::provide` or bad things will happen
pub(crate) type ProvideFn<'cx, S> =
    unsafe fn(*const (), &'cx Context) -> <S as Service>::Output<'cx>;

/// An object that can be created by a [`Provider`] and stored in a [`Context`]. The type this trait
/// is implemented on acts as the key that distinguishes services from eachother.
///
/// In most cases, an implementation of this trait is trivial, and so it is recommended to simply
/// derive it.
pub trait Service: 'static {
    /// The result of a service resolution
    type Output<'cx>;
}
