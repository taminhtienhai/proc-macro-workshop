use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::{
    Attribute, Error, Field, GenericArgument, LitStr, Meta, Path, PathArguments, PathSegment,
    Result, Token, Type, TypePath, Expr,
};

pub struct FieldMetadata {
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub ty: Type,
}

pub enum TypeDef {
    Option(Type),
    Vec(Type, Type),
    Other(Type),
}

impl From<&Field> for FieldMetadata {
    fn from(
        Field {
            attrs, ident, ty, ..
        }: &Field,
    ) -> Self {
        FieldMetadata {
            attrs: attrs.clone(),
            ident: ident.clone().unwrap(),
            ty: ty.clone(),
        }
    }
}

impl FieldMetadata {
    pub fn typedef(&self) -> TypeDef {
        match &self.ty {
            Type::Path(TypePath {
                path: Path { segments, .. },
                ..
            }) => {
                if let Some(PathSegment {
                    ident,
                    arguments: PathArguments::AngleBracketed(angle),
                }) = segments.last()
                {
                    if let (1, Some(GenericArgument::Type(t))) =
                        (angle.args.len(), angle.args.first())
                    {
                        if ident == "Option" && angle.args.len() == 1 {
                            return TypeDef::Option(t.clone());
                        }
                        if ident == "Vec" && angle.args.len() == 1 {
                            return TypeDef::Vec(self.ty.clone(), t.clone());
                        }
                    }
                }
                return TypeDef::Other(self.ty.clone());
            }
            _ => TypeDef::Other(self.ty.clone()),
        }
    }

    pub fn get_ident(&self) -> Result<Option<Ident>> {
        let mut f_ident: Option<Ident> = None;
        let mut err = Ok(());
        for attr in &self.attrs {
            if attr.path().is_ident("builder") {
                err = attr.parse_nested_meta(|meta| {
                    if !meta.path.is_ident("each") {
                        return Err(meta.error("expected `builder(each = \"...\")`"))
                    }

                    let equal = meta.input.parse::<Token![=]>();
                    eprintln!("Equal of attr: {equal:?}");
                    let value = meta.input.parse::<LitStr>();
                    eprintln!("Value of attr: {value:?}");


                    if let (Ok(_), Ok(fn_ident)) = (equal, value) {
                        f_ident = Some(Ident::new(fn_ident.value().as_str(), fn_ident.span()));
                        Ok(())
                    } else {
                        Err(meta.error("expected `builder(each = \"...\")`"))
                    }
                });
                break;
            }
        }
    
        err.and(Ok(f_ident))
    }

    pub fn impl_attr_setter_fns(&self) -> Result<TokenStream> {
        let mut r_ident = self.ident.clone();
        let typedef = self.typedef();
        let mut value_type = typedef.get_attr_type();

        let assign_value_block = match typedef {
            TypeDef::Vec(_, ty) => {
                eprintln!("Enter build vec fn");
                if let Some(id) = self.get_ident()? {
                    let ide = r_ident.clone();
                    r_ident = id;
                    value_type = ty;
                    quote! {
                        self.#ide.as_mut().map(|i| i.push(value));
                    }
                } else {
                    quote! {
                        self.#r_ident = Some(value);
                    }
                }
            }
            TypeDef::Option(_) => quote! {
                self.#r_ident = Some(Some(value));
            },
            _ => quote! {
                self.#r_ident = Some(value);
            },
        };

        Ok(quote! {
            pub fn #r_ident(&mut self, value: #value_type) -> &mut Self {
                #assign_value_block
                self
            }
        }
        .into())
    }
}

impl TypeDef {
    pub fn get_attr_type(&self) -> Type {
        match self {
            Self::Vec(rt, _) => rt.clone(),
            Self::Option(t) => t.clone(),
            Self::Other(t) => t.clone(),
        }
    }
}
