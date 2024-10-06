extern crate proc_macro;
use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericParam, Generics, Type};

#[proc_macro_derive(BookSide)]
pub fn derive_book_side(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = generate_book_side_impl(&input);
    TokenStream::from(expanded)
}

fn generate_book_side_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    let Data::Struct(data_struct) = &input.data else {
        panic!("#[derive(BookSide)] can only be used with structs");
    };

    let Fields::Named(fields_named) = &data_struct.fields else {
        panic!("BookSide can only be derived for structs with named fields");
    };

    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    validate_generics(&generics);

    let has_levels = fields_named.named.iter().any(|f| {
        f.ident
            .as_ref()
            .map(|ident| ident == "levels")
            .unwrap_or(false)
    });

    if has_levels {
        quote! {
            impl #impl_generics order_book_core::book_side::BookSide<Px, Qty> for #name #ty_generics #where_clause {
                fn levels(&self) -> &hashbrown::HashMap<Px, Qty> {
                    &self.levels
                }
                fn levels_mut(&mut self) -> &mut hashbrown::HashMap<Px, Qty> {
                    &mut self.levels
                }
            }
        }
    } else {
        panic!("Struct {} does not have a `levels` field", name);
    }
}

#[proc_macro_derive(BidAskBook)]
pub fn derive_bid_ask_book(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = generate_bid_ask_book_impl(&input);
    TokenStream::from(expanded)
}

fn generate_bid_ask_book_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    validate_generics(&generics);

    let Data::Struct(data_struct) = &input.data else {
        panic!("BidAskBook can only be derived for structs");
    };
    let Fields::Named(fields_named) = &data_struct.fields else {
        panic!("BidAskBook can only be derived for structs with named fields");
    };

    let bids_type = find_field_type(fields_named, "bids");
    let asks_type = find_field_type(fields_named, "asks")
        .expect("Struct does not have a required `asks` field");

    quote! {
        impl #impl_generics order_book_core::order_book::BidAskBook<Px, Qty> for #name #ty_generics #where_clause {
            type AskBookSide = #asks_type;
            type BidBookSide = #bids_type;

            fn asks(&self) -> &Self::AskBookSide {
                &self.asks
            }

            fn bids(&self) -> &Self::BidBookSide {
                &self.bids
            }

            fn asks_mut(&mut self) -> &mut Self::AskBookSide {
                &mut self.asks
            }

            fn bids_mut(&mut self) -> &mut Self::BidBookSide {
                &mut self.bids
            }
        }
    }
}

fn find_field_type(fields_named: &syn::FieldsNamed, field_name: &str) -> Option<Type> {
    fields_named.named.iter().find_map(|field| {
        if let Some(ident) = &field.ident {
            if ident == field_name {
                return Some(field.ty.clone());
            }
        }
        None
    })
}

fn validate_generics(generics: &Generics) {
    let mut generic_iter = generics.params.iter();
    match generic_iter.next() {
        Some(GenericParam::Type(ty)) if ty.ident == "Px" => {}
        _ => {
            panic!("BidAskBook can only be derived for structs where the first type parameter is named `Px`");
        }
    }
    match generic_iter.next() {
        Some(GenericParam::Type(ty)) if ty.ident == "Qty" => {}
        _ => {
            panic!("BidAskBook can only be derived for structs where the second type parameter is named `Qty`");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, DeriveInput};

    #[test]
    fn test_generate_book_side_impl() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct<Px, Qty> {
                levels: hashbrown::HashMap<Px, Qty>,
            }
        };

        let expanded = generate_book_side_impl(&input);
        let expected = quote! {
            impl<Px, Qty> order_book_core::book_side::BookSide<Px, Qty> for TestStruct<Px, Qty> {
                fn levels(&self) -> &hashbrown::HashMap<Px, Qty> {
                    &self.levels
                }
                fn levels_mut(&mut self) -> &mut hashbrown::HashMap<Px, Qty> {
                    &mut self.levels
                }
            }
        };

        assert_eq!(expanded.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_bid_ask_book_impl() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct<Px, Qty> {
                bids: SomeType<Px, Qty>,
                asks: AnotherType<Px>,
            }
        };

        let expanded = generate_bid_ask_book_impl(&input);
        let expected = quote! {
            impl<Px, Qty> order_book_core::order_book::BidAskBook<Px, Qty> for TestStruct<Px, Qty> {
                type AskBookSide = AnotherType<Px>;
                type BidBookSide = SomeType<Px, Qty>;

                fn asks(&self) -> &Self::AskBookSide {
                    &self.asks
                }

                fn bids(&self) -> &Self::BidBookSide {
                    &self.bids
                }

                fn asks_mut(&mut self) -> &mut Self::AskBookSide {
                    &mut self.asks
                }

                fn bids_mut(&mut self) -> &mut Self::BidBookSide {
                    &mut self.bids
                }
            }
        };

        assert_eq!(expanded.to_string(), expected.to_string());
    }
}
