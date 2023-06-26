use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Serialize)]
pub fn epserde_serialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // We have to play with this to get type parameters working
    let mut generics = quote!();
    let mut generics_names = quote!();
    if !input.generics.params.is_empty() {
        input.generics.params.iter().for_each(|x| {
            match x {
                syn::GenericParam::Type(t) => {
                    generics_names.extend(t.ident.to_token_stream());
                }
                syn::GenericParam::Lifetime(l) => {
                    generics_names.extend(l.lifetime.to_token_stream());
                }
                syn::GenericParam::Const(c) => {
                    generics_names.extend(c.ident.to_token_stream());
                }
            };
            generics_names.extend(quote!(,))
        });

        input.generics.params.into_iter().for_each(|x| match x {
            syn::GenericParam::Type(t) => {
                let mut t = t.clone();
                t.bounds.push(syn::TypeParamBound::Trait(syn::TraitBound {
                    paren_token: None,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: None,
                    path: syn::parse_quote!(epserde_trait::Serialize),
                }));
                generics.extend(quote!(#t,));
            }
            x => {
                generics.extend(x.to_token_stream());
                generics.extend(quote!(,))
            }
        });
    }

    let where_clause = input
        .generics
        .where_clause
        .map(|x| x.to_token_stream())
        .unwrap_or(quote!(""));

    let out = match input.data {
        Data::Struct(s) => {
            let fields = s.fields.into_iter().map(|field| field.ident.unwrap());
            quote! {
                #[automatically_derived]
                impl<#generics> epserde_trait::Serialize for #name<#generics_names> #where_clause {
                    fn serialize<F: std::io::Write + std::io::Seek>(&self, backend: &mut F) -> anyhow::Result<usize> {
                        let mut bytes = 0;
                        bytes += Self::write_endianness_marker(backend)?;
                        #(
                            bytes += self.#fields.serialize(backend)?;

                        )*
                        Ok(bytes)
                    }
                }
            }
        }
        _ => todo!(),
    };
    out.into()
}

#[proc_macro_derive(Deserialize)]
pub fn epserde_deserialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    // We have to play with this to get type parameters working
    let mut generics = quote!();
    let mut generics_names = quote!();
    if !input.generics.params.is_empty() {
        input.generics.params.iter().for_each(|x| {
            match x {
                syn::GenericParam::Type(t) => {
                    generics_names.extend(t.ident.to_token_stream());
                }
                syn::GenericParam::Lifetime(l) => {
                    generics_names.extend(l.lifetime.to_token_stream());
                }
                syn::GenericParam::Const(c) => {
                    generics_names.extend(c.ident.to_token_stream());
                }
            };

            generics_names.extend(quote!(,))
        });

        input.generics.params.into_iter().for_each(|x| match x {
            syn::GenericParam::Type(t) => {
                let mut t = t.clone();
                t.bounds.push(syn::TypeParamBound::Trait(syn::TraitBound {
                    paren_token: None,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: None,
                    path: syn::parse_quote!(epserde_trait::Deserialize<'epserde_deserialize>),
                }));
                generics.extend(quote!(#t,));
            }
            x => {
                generics.extend(x.to_token_stream());
                generics.extend(quote!(,))
            }
        });
    }

    let where_clause = input
        .generics
        .where_clause
        .map(|x| x.to_token_stream())
        .unwrap_or(quote!(""));

    let out = match input.data {
        Data::Struct(s) => {
            let fields = s
                .fields
                .iter()
                .map(|field| field.ident.to_owned().unwrap())
                .collect::<Vec<_>>();

            let types = s.fields.into_iter().map(|field| field.ty);

            quote! {
                #[automatically_derived]
                impl<'epserde_deserialize, #generics> epserde_trait::Deserialize<'epserde_deserialize> for #name<#generics_names> #where_clause{
                    fn deserialize(backend: &'epserde_deserialize [u8]) -> anyhow::Result<(Self, &'epserde_deserialize [u8])> {
                        let mut bytes = 0;
                        let backend = Self::check_endianness_marker(backend)?;
                        #(
                            let(#fields, backend) = #types::deserialize(backend)?;
                        )*
                        Ok((#name{
                            #(#fields),*
                        }, backend))
                    }
                }
            }
        }
        _ => todo!(),
    };
    out.into()
}
