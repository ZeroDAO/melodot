extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, ImplItem, Visibility, PathSegment};
use syn::spanned::Spanned;

/// Generates an implementation block with optional types: RuntimeEvent, WeightInfo, and Currency.
#[proc_macro_attribute]
pub fn auto_config(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input and arguments
    let mut item: ItemImpl = parse_macro_input!(input as ItemImpl);
    let mut include_event = true;
    let mut include_weight = true;
    let mut include_currency = false;

    // Analyze attributes to determine whether to include optional types
    for arg in args.into_iter() {
        match arg.to_string().as_str() {
            "skip_event" => include_event = false,
            "skip_weight" => include_weight = false,
            "include_currency" => include_currency = true,
            _ => {},
        }
    }

    // Automatically extract the pallet name from the trait path
    let path = &item.trait_.as_ref().expect("Expected a trait impl").1;
    let segment: &PathSegment = path.segments.first().expect("Expected a path segment");
    let pallet_name = segment.ident.to_string();

    // Prepare additional ImplItems
    let mut additional_items = vec![];

    if include_event {
        additional_items.push(generate_impl_item_type("RuntimeEvent", "RuntimeEvent", Visibility::Inherited));
    }

    if include_weight {
        let weight_type_str = format!("{}::weights::SubstrateWeight<Runtime>", pallet_name);
        additional_items.push(generate_impl_item_type(&weight_type_str, "WeightInfo", Visibility::Inherited));
    }

    if include_currency {
        additional_items.push(generate_impl_item_type("Balances", "Currency", Visibility::Inherited));
    }

    // Extend the existing impl block
    item.items.extend(additional_items);

    // Generate the final token stream
    TokenStream::from(quote! {
        #item
    })
}

fn generate_impl_item_type(
    type_name: &str,
    field_name: &str,
    visibility: Visibility,
) -> ImplItem {
    let parsed_type: syn::Type = syn::parse_str(type_name).unwrap();
    let type_span = parsed_type.span();
    
    ImplItem::Type(syn::ImplItemType {
        attrs: vec![],
        vis: visibility,
        defaultness: None,
        type_token: syn::Token![type](type_span),
        ident: syn::Ident::new(field_name, type_span),
        generics: Default::default(),
        eq_token: syn::Token![=](type_span),
        ty: parsed_type,
        semi_token: syn::Token![;](type_span),
    })
}
