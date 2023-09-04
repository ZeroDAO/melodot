// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Melo Auto Config
//! 
//! This crate provides a procedural macro to automatically generate boilerplate code for Substrate pallet configuration traits.
//! 
//! ## Example Usage
//! 
//! ```rust
//! use melo_auto_config::auto_config;
//! 
//! #[auto_config]
//! impl pallet_balances::Config for Runtime {
//!     type MaxLocks = ConstU32<50>;
//!     // other types...
//! }
//! ```
//! 
//! You can add the following attributes to enable or disable features:
//! - `skip_event`: Skips the generation of `RuntimeEvent` type
//! - `skip_weight`: Skips the generation of `WeightInfo` type
//! - `include_currency`: Includes the generation of `Currency` type
//! 
//! ```rust
//! #[auto_config(skip_event, include_currency)]
//! impl pallet_balances::Config for Runtime {
//!     // ...
//! }
//! ```
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, ImplItem, Visibility, PathSegment};
use syn::spanned::Spanned;

/// This attribute macro generates additional types for the Substrate pallet configuration trait.
///
/// You can use `skip_event` to skip `RuntimeEvent` type generation, `skip_weight` to skip `WeightInfo` type generation, and `include_currency` to include `Currency` type.
///
/// # Example
///
/// ```ignore
/// #[auto_config]
/// impl pallet_balances::Config for Runtime {
///     type MaxLocks = ConstU32<50>;
/// }
/// ```
///
/// This will automatically generate:
/// 
/// ```ignore
/// type RuntimeEvent = RuntimeEvent;
/// type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
/// ```
///
/// The `RuntimeEvent` and `WeightInfo` types are automatically generated and added to your trait implementation.
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

// A helper function to generate a new `ImplItem::Type`.
//
// # Arguments
//
// * `type_name` - The name of the type as a string.
// * `field_name` - The name of the field in the impl.
// * `visibility` - The visibility of the type.
//
// # Returns
//
// Returns a new `ImplItem::Type` that can be added to an existing `ItemImpl`.
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