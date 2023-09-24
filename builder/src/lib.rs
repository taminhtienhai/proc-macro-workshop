mod builder_macro_v2;

use builder_macro_v2::FieldMetadata;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, AngleBracketedGenericArguments,
    DeriveInput, Error, Field, GenericArgument, GenericParam, Generics, Path, PathArguments,
    PathSegment, Type, TypePath,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn impl_struct_builder(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    match struct_builder(derive_input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

fn struct_builder(
    DeriveInput {
        data,
        generics,
        ident,
        ..
    }: DeriveInput,
) -> syn::Result<TokenStream2> {
    let Generics {
        params,
        where_clause,
        ..
    } = &generics;

    let gen_idents = params
        .iter()
        .filter(|it| match it {
            GenericParam::Lifetime(_) => false,
            _ => true,
        })
        .map(|it| match it {
            GenericParam::Type(ty) => ty.ident.clone(),
            GenericParam::Const(cons) => cons.ident.clone(),
            _ => panic!("Lifetime does not allow here"),
        })
        .fold(Punctuated::<Ident, Comma>::default(), |mut acc, cur| {
            acc.push(cur);
            acc
        });


    let props = match data {
        syn::Data::Struct(body) => match body.fields {
            syn::Fields::Named(fields) => fields.named,
            _ => return Err(Error::new(Span::call_site(), "Only support named property")),
        },
        _ => return Err(Error::new(Span::call_site(), "support struct only")),
    };
    let builder_ident = format_ident!("{}Builder", ident);

    let field_metadata: Vec<FieldMetadata> = props
        .iter()
        .map(|field| field.into())
        .collect();

    let builder_fns_v2 = field_metadata
        .into_iter()
        .map(|it| it.impl_attr_setter_fns().unwrap())
        .collect::<Vec<_>>();

    let builder_props = props
        .iter()
        .map(|Field { ident, ty, .. }| {
            quote!(#ident: core::option::Option<#ty>)
        });
    let _builder_fns = props.iter().map(|Field { ident, ty, .. }| {
        match check_ty(ty) {
            TypeInfer::Option(t) => {
                quote! {
                    pub fn #ident(&mut self, value: #t) -> &mut Self {
                        self.#ident = Some(Some(value));
                        self
                    }
                }
            },
            _ => quote! {
                pub fn #ident(&mut self, value: #ty) -> &mut Self {
                    self.#ident = Some(value);
                    self
                }
            }
        }
    });
    let init_default_props = props
        .iter()
        .map(|Field { vis, ident, ty, .. }| match check_ty(ty) {
            TypeInfer::Option(_) => {
                quote!(#vis #ident: Some(None))
            },
            TypeInfer::Vec => {
                quote!(#vis #ident: Some(vec![]))
            },
            _ => quote!(#vis #ident: None),
        });
    let build_props = props.iter().map(|Field { ident, .. }| {
        quote!(
            #ident: self.#ident.take().ok_or(format!("`{}` is required", stringify!(#ident)))?
        )
    });

    Ok(quote! {
        impl <#params> #ident <#gen_idents> #where_clause {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#init_default_props, )*
                }
            }
        }

        pub struct #builder_ident <#params> #where_clause {
            #(#builder_props,)*
        }

        impl <#params> #builder_ident <#gen_idents> #where_clause {
            #(#builder_fns_v2)*

            pub fn build(&mut self) -> core::result::Result<#ident, Box<dyn std::error::Error>> {
                Ok(#ident {
                    #(#build_props,)*
                })
            }
        }
    }
    .into())
}

fn check_ty(ty: &Type) -> TypeInfer {
    match ty {
        Type::Path(TypePath {
            qself: _,
            path:
                Path {
                    segments,
                    leading_colon: _,
                },
        }) => if let Some(PathSegment {
            ident,
            arguments:
                PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }),
        }) = segments.last()
        {
            let mut r_ty = TypeInfer::Other;
            if let (1, Some(GenericArgument::Type(t))) = (args.len(), args.first()) {
                if ident == "Option" {
                    r_ty = TypeInfer::Option(t.clone());
                } else if ident == "Vec" {
                    r_ty = TypeInfer::Vec;
                }
            }
            r_ty
        } else {
            TypeInfer::Other
        },
        _ => TypeInfer::Other,
    }
}

enum TypeInfer {
    Option(Type),
    Vec,
    Other,
}
