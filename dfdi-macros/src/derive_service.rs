use proc_macro2::{Span, TokenStream};
use proc_macro_crate::FoundCrate;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::{Pair, Punctuated},
    spanned::Spanned,
    token::Paren,
    visit_mut::{visit_type_path_mut, VisitMut},
    AngleBracketedGenericArguments, DeriveInput, Expr, ExprPath, GenericArgument, GenericParam,
    Generics, Ident, Lifetime, Path, PathSegment, Result, Token, Type, TypePath, TypeTuple,
};

/// Parsed #[service(Argument -> Output)] attribute
struct ServiceAttr {
    arg: Type,
    out: Type,
}

impl Parse for ServiceAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let arg = Type::parse(input)?;
        input.parse::<Token![->]>()?;
        let out = Type::parse(input)?;

        Ok(Self { arg, out })
    }
}

pub fn derive_service(input: DeriveInput) -> Result<TokenStream> {
    // Parse the #[service] attribute
    let mut service_attr = None;
    for attr in input
        .attrs
        .into_iter()
        .filter(|attr| attr.path.is_ident("service"))
    {
        if service_attr.is_some() {
            return Err(syn::Error::new(
                attr.path.span(),
                "Duplicate service attribute",
            ));
        }

        service_attr = Some(attr.parse_args::<ServiceAttr>()?);
    }

    // Find the path to the `Service` trait
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

    // Build the TypePath refering to this type
    let ty = build_type_path(input.ident, &input.generics);

    // The types requested by the user for the argument and output.
    let (mut arg_ty, mut out_ty) = match service_attr {
        Some(attr) => (attr.arg, attr.out),
        None => (
            // An empty tuple
            Type::Tuple(TypeTuple {
                paren_token: Paren {
                    span: Span::call_site(),
                },
                elems: Punctuated::new(),
            }),
            // The `Self` type, as interpreted in the attribute.
            Type::Path(ty.clone()),
        ),
    };

    // Patch the Output type:
    // - Replace all non-'static lifetimes with 'cx
    // - Replace all instances of the `Self` type with the default output type
    ServiceTypeVisitor::new(Some(ty.clone()), Lifetime::new("'cx", Span::call_site()))
        .visit(&mut out_ty)?;

    // Patch the Argument type:
    // - Replace all non-'static lifetimes with 'arg
    // - Forbid using `Self` which makes little sense here
    ServiceTypeVisitor::new(None, Lifetime::new("'arg", Span::call_site())).visit(&mut arg_ty)?;

    // Replace all lifetimes on the original type with 'static to create the type Service will be
    // implemented on.
    let mut service_ty = Type::Path(ty);
    ServiceTypeVisitor::new(None, Lifetime::new("'static", Span::call_site()))
        .visit(&mut service_ty)?;

    // Remove the lifetimes from the type parameters, since they will not be generic
    let mut ty_params = input.generics;
    ty_params.params = ty_params
        .params
        .into_pairs()
        .filter(|pair| !matches!(pair.value(), GenericParam::Lifetime(_)))
        .collect();

    // Final impl
    let expanded = quote! {
        impl #ty_params #service_trait for #service_ty {
            type Output<'cx> = #out_ty;
            type Argument<'arg> = #arg_ty;
        }
    };

    Ok(expanded)
}

/// - Replace non-'static lifetimes with the provider lifetime
/// - Replace `Self` with the supplied type, or produce an error if self_ty is None
struct ServiceTypeVisitor {
    lifetime: Lifetime,
    self_ty: Option<TypePath>,

    error: Option<syn::Error>,
}

impl ServiceTypeVisitor {
    fn new(self_ty: Option<TypePath>, lifetime: Lifetime) -> Self {
        Self {
            self_ty,
            lifetime,
            error: None,
        }
    }

    fn visit(&mut self, ty: &mut Type) -> Result<()> {
        self.visit_type_mut(ty);
        match self.error.take() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    fn with_error(&mut self, err: syn::Error) {
        self.error = Some(match self.error.take() {
            Some(mut other) => {
                other.combine(err);
                other
            }
            None => err,
        })
    }
}

impl VisitMut for ServiceTypeVisitor {
    fn visit_type_path_mut(&mut self, i: &mut TypePath) {
        if i.path.get_ident().map(|i| i == "Self").unwrap_or(false) {
            if let Some(ref self_ty) = self.self_ty {
                *i = self_ty.clone();
            } else {
                return self.with_error(syn::Error::new(i.span(), "`Self` is not allowed here"));
            }
        }

        visit_type_path_mut(self, i);
    }

    fn visit_lifetime_mut(&mut self, i: &mut Lifetime) {
        if i.ident != "'static" {
            *i = self.lifetime.clone();
        }
    }
}

fn build_type_path(ident: Ident, generics: &Generics) -> TypePath {
    let args = generics
        .params
        .pairs()
        .into_iter()
        .map(|p| {
            let (val, sep) = p.into_tuple();

            let val = match val {
                GenericParam::Type(param) => GenericArgument::Type(Type::Path(TypePath {
                    qself: None,
                    path: Path {
                        leading_colon: None,
                        segments: [PathSegment {
                            ident: param.ident.clone(),
                            arguments: syn::PathArguments::None,
                        }]
                        .into_iter()
                        .collect(),
                    },
                })),
                GenericParam::Const(param) => GenericArgument::Const(Expr::Path(ExprPath {
                    attrs: Vec::new(),
                    qself: None,
                    path: Path {
                        leading_colon: None,
                        segments: [PathSegment {
                            ident: param.ident.clone(),
                            arguments: syn::PathArguments::None,
                        }]
                        .into_iter()
                        .collect(),
                    },
                })),
                GenericParam::Lifetime(param) => GenericArgument::Lifetime(param.lifetime.clone()),
            };

            Pair::new(val, sep.cloned())
        })
        .collect::<Punctuated<GenericArgument, Token![,]>>();

    TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: [PathSegment {
                ident,
                arguments: syn::PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    args,
                    colon2_token: None,
                    lt_token: Token![<](Span::call_site()),
                    gt_token: Token![>](Span::call_site()),
                }),
            }]
            .into_iter()
            .collect(),
        },
    }
}
