#![forbid(unsafe_code)]

mod derive_service;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

/// Create an implementation of [`Service`] on a `'static` version of the original type.
///
/// You can use the `#[service(Argument -> Output)]` attribute to customize the argument and return
/// types. The default service attribute is `#[service(() -> Self)]`.
///
/// To produce the final impl, the derive macro follows these steps:
/// - Replace all the lifetimes on the type with `'static` and implement `Service` on the new type
/// - Set `Output<'cx>` to the output type with all non-'static lifetimes replaced by `'cx`
/// - Set `Argument<'arg>` to the argument type with all non-'static lifetimes replaced by `'arg`
///
/// ```
/// # use dfdi::Service;
/// #[derive(Service)]
/// struct Ref<'a, T>(&'a T);
///
/// // The above generates:
/// // impl<T> Service for Ref<'static, T> {
/// //    type Output<'cx> = Ref<'cx, T>;
/// //    type Argument<'arg> = ();
/// // }
///
/// #[derive(Service)]
/// #[service(bool -> Option<Self>)]
/// struct MaybeRef<'a, T>(&'a T);
///
/// // The above generates:
/// // impl<T> Service for MaybeRef<'static, T> {
/// //    type Output<'cx> = Option<MaybeRef<'cx, T>>;
/// //    type Argument<'arg> = bool;
/// // }
/// ```
#[proc_macro_derive(Service, attributes(service))]
pub fn derive_service(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_service::derive_service(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
