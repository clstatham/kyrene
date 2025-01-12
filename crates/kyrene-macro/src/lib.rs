use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

// port of `tokio::main`
#[proc_macro_attribute]
pub fn main(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _args: TokenStream = args.into();
    let mut item: TokenStream = item.into();

    let input: ItemFn = match syn::parse2(item.clone()) {
        Ok(it) => it,
        Err(e) => {
            item.extend(e.into_compile_error());
            return item.into();
        }
    };

    let body = input.block;

    quote! {
        fn main() {
            ::kyrene::core::tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    #body
                })
        }
    }
    .into()
}
