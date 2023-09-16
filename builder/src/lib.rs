mod field_parser;
mod struct_parser;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, Generics, GenericParam, punctuated::Punctuated, Token, token::Comma, braced, Field};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput {
        data,
        generics,
        ident,
        ..
    } = parse_macro_input!(input as DeriveInput);

    let Generics {
        params,
        where_clause,
        ..
    } = &generics;

    let gen_idents = params.iter()
        .filter(|it| match it {
            GenericParam::Lifetime(_) => false,
            _ => true,
        })
        .map(|it| match it {
            GenericParam::Type(ty) => ty.ident.clone(),
            GenericParam::Const(cons) => cons.ident.clone(),
            _ => panic!("")
        })
        .fold(Punctuated::<Ident, Comma>::default(), |mut acc, cur| {
            acc.push(cur);
            acc
        });

    let fields = match data {
        syn::Data::Struct(body) => match body.fields {
            syn::Fields::Named(fields) => fields.named,
            _ => panic!("Only support named fields"),
        },
        _ => panic!("Not a struct"),
    };
    let builder_ident = format_ident!("{}Builder", ident);


    let builder_fns = fields.iter()
        .map(|Field { ident, ty, .. }| {
            quote! {
                pub fn #ident(&mut self, value: #ty) {
                    self.#ident = value;
                }
            }
        });
    let init_default_props = fields.iter()
        .map(|Field { vis, ident, .. }| {
            quote!(#vis #ident: core::default::Default::default())
        });
    let build_props = fields.iter()
        .map(|Field { ident, .. }| {
            quote!(#ident: self.#ident)
        });
    quote! {
        impl <#params> #ident <#gen_idents> #where_clause {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#init_default_props, )*
                }
            }
        }
        
        pub struct #builder_ident <#params> #where_clause {
            #fields
        }

        impl <#params> #builder_ident <#gen_idents> #where_clause {
            #(#builder_fns)*

            pub fn build(self) -> core::result::Result<#ident, Box<dyn std::error::Error>> {
                Ok(#ident {
                    #(#build_props,)*
                })
            }
        }
    }.into()
}
