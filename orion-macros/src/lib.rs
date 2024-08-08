use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Stmt};

#[proc_macro_attribute]
pub fn init_tracing(_: TokenStream, item: TokenStream) -> TokenStream {
    // eprintln!("inititem: {:#?}", item);
    let mut item_fn = parse_macro_input!(item as syn::ItemFn);
    // eprintln!("inititemfn: {:#?}", item_fn);
    let stmts = &mut item_fn.block.stmts;
    let stmt1: Stmt = syn::parse2(
        quote! {
            let collector = tracing_subscriber::fmt()
            // filter spans/events with level TRACE or higher.
            .with_max_level(tracing::Level::INFO)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ENTER | tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            // build but do not install the subscriber.
            .finish();
        }
    )
    .unwrap();
    let stmt2: Stmt = syn::parse2(
        quote! {
            tracing::subscriber::set_global_default(collector).expect("setting default subscriber failed");
        }
    )
    .unwrap();
    stmts.insert(0, stmt2);
    stmts.insert(0, stmt1);
    item_fn.to_token_stream().into()
}

// #[proc_macro_attribute]
// pub fn asshole(_: TokenStream, item: TokenStream) -> TokenStream {
//     eprintln!("itemtokens: {:#?}", item);
//     let item_fn = parse_macro_input!(item as syn::ItemFn);
//     eprintln!("itemfn: {:#?}", item_fn);
//     quote! {}.into()
// }
