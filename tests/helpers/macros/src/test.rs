use proc_macro2::TokenStream;
use quote::quote;

pub fn expand(input: syn::ItemFn) -> TokenStream {
    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let inputs = &input.sig.inputs;
    let body = &input.block;
    let attrs = &input.attrs;

    quote! {
        #[sqlx::test]
        #(#attrs)*
        async fn #name(pool: test_server::DbPool) #ret {
            async fn inner(#inputs) #ret {
                #body
            }
            inner(test_server::TestServer::run(pool).await).await
        }
    }
}
