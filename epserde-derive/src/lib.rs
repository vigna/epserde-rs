use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Serialize)]
pub fn epserde_serialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let out = match input.data {
        Data::Struct(s) => {
            let fields = s.fields.into_iter().map(|field| field.ident.unwrap());
            quote! {
                #[automatically_derived]
                impl epserde_trait::Serialize for #name {
                    fn serialize<F: std::io::Write + std::io::Seek>(&self, backend: &mut F) -> anyhow::Result<usize> {
                        let mut bytes = 0;
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
    let generics = input.generics.params.into_iter().collect::<Vec<_>>();

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
                impl<'a> epserde_trait::Deserialize<'a> for #name {
                    fn deserialize(backend: &'a [u8]) -> anyhow::Result<(Self, &'a [u8])> {
                        let mut bytes = 0;
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
