//! Pass 8 — tokenize: concatenate the emitted item bodies into the final
//! `TokenStream` the proc-macro entrypoint returns.
//!
//! Hygiene and ident resolution already happened upstream in lowering; this pass
//! only concatenates. Items are emitted in source (`EmittedItemId`) order, which
//! is exactly the order [`crate::emit::finalize`] leaves `model.items` in.

use crate::emit::EmittedItemModel;
use proc_macro2::TokenStream;

/// Concatenate every item's assembled tokens, in source order, into one stream.
#[must_use]
pub fn to_tokens(model: &EmittedItemModel) -> TokenStream {
    let mut stream = TokenStream::new();
    for item in &model.items {
        stream.extend(item.node.to_item_tokens());
    }
    stream
}
