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
