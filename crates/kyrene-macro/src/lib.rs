use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

#[proc_macro_derive(Bundle)]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = match syn::parse(input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let ident = &input.ident;
    let (ig, tg, wc) = &input.generics.split_for_impl();

    let mut members = Vec::new();
    let mut member_types = Vec::new();
    match input.data {
        syn::Data::Struct(ref data_struct) => {
            for field in data_struct.fields.iter() {
                members.push(field.ident.as_ref().unwrap());
                member_types.push(&field.ty);
            }
        }
        _ => {
            return syn::Error::new(input.span(), "Expected struct")
                .to_compile_error()
                .into()
        }
    };

    quote! {
        impl #ig kyrene_core::bundle::Bundle for #ident #tg #wc {
            fn into_dyn_components(self) -> Vec<(kyrene_core::util::TypeInfo, Box<dyn kyrene_core::component::Component>)> {
                vec![#(
                    (kyrene_core::util::TypeInfo::of::<#member_types>(), Box::new(self.#members))
                ),*]
            }
        }
    }
    .into()
}
