use proc_macro2::{Span, TokenStream};
use proc_macro_crate::FoundCrate;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::{Pair, Punctuated},
    spanned::Spanned,
    DeriveInput, GenericParam, Generics, Ident, Lifetime, Result, Token,
};

struct ServiceArg {
    key: Ident,
    kind: ServiceArgKind,
}

enum ServiceArgKind {
    Error(syn::Type),
}

impl ServiceArg {
    /// Parse a comma separated list of service arguments
    fn parse_list(input: ParseStream) -> Result<impl Iterator<Item = ServiceArg>> {
        Ok(Punctuated::<ServiceArg, Token![,]>::parse_separated_nonempty(input)?.into_iter())
    }
}

impl Parse for ServiceArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = Ident::parse(input)?;

        let kind = if key == "error" {
            let _ = <syn::Token![=]>::parse(input)?;
            let ty = syn::Type::parse(input)?;
            ServiceArgKind::Error(ty)
        } else {
            return Err(syn::Error::new(key.span(), "invalid key: expected `error`"));
        };

        Ok(Self { key, kind })
    }
}

#[derive(Default)]
struct ServiceArgs {
    error: Option<syn::Type>,
}

impl ServiceArgs {
    fn set_arg(&mut self, arg: ServiceArg) -> Result<()> {
        match arg.kind {
            ServiceArgKind::Error(ty) => {
                if self.error.is_some() {
                    return Err(syn::Error::new(
                        arg.key.span(),
                        "Duplicate definition of `error`",
                    ));
                }

                self.error = Some(ty);
            }
        };

        Ok(())
    }
}

pub fn derive_service(input: DeriveInput) -> Result<TokenStream> {
    let mut args = ServiceArgs::default();
    for attr in input
        .attrs
        .into_iter()
        .filter(|attr| attr.path.is_ident("service"))
    {
        let arg_list = attr.parse_args_with(ServiceArg::parse_list)?;
        for arg in arg_list {
            args.set_arg(arg)?;
        }
    }

    let service_trait = match proc_macro_crate::crate_name("dfdi")
        .or_else(|_| proc_macro_crate::crate_name("dfdi-core"))
        .map_err(|_| {
            syn::Error::new(
                Span::call_site(),
                "Crate `dfdi` or `dfdi-core` must be present in Cargo.toml",
            )
        })? {
        FoundCrate::Itself => quote!(dfdi::Service),
        FoundCrate::Name(name) => {
            let name = Ident::new(&name, Span::call_site());
            quote!(#name::Service)
        }
    };

    let ty = input.ident;

    fn replace_lifetimes(generics: Generics, with: &'static str) -> TokenStream {
        let punct = generics
            .params
            .into_pairs()
            .into_iter()
            .map(|p| {
                let (val, sep) = p.into_tuple();

                let val = match val {
                    GenericParam::Type(param) => param.ident.to_token_stream(),
                    GenericParam::Const(param) => param.ident.to_token_stream(),
                    GenericParam::Lifetime(param) => {
                        Lifetime::new(with, param.span()).to_token_stream()
                    }
                };

                Pair::new(val, sep)
            })
            .collect::<Punctuated<TokenStream, Token![,]>>();
        if punct.empty_or_trailing() {
            quote!()
        } else {
            quote!(<#punct>)
        }
    }

    // Generics with all lifetimes replaced with 'cx
    let cx_lt_generics = replace_lifetimes(input.generics.clone(), "'cx");

    // Generics with all lifetimes replaced with 'static
    let static_lt_generics = replace_lifetimes(input.generics.clone(), "'static");

    // Generics without lifetimes
    let mut ty_params = input.generics;
    ty_params.params = ty_params
        .params
        .into_pairs()
        .filter(|pair| !matches!(pair.value(), GenericParam::Lifetime(_)))
        .collect();

    // The final output type
    let mut output_ty = quote!(#ty #cx_lt_generics);
    if let Some(error_ty) = args.error {
        output_ty = quote!(core::result::Result<#output_ty, #error_ty>);
    }

    let expanded = quote! {
        impl #ty_params #service_trait for #ty #static_lt_generics {
            type Output<'cx> = #output_ty;
            type Argument<'arg> = ();
        }
    };

    Ok(expanded)
}
