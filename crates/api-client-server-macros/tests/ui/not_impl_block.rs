use api_macros::api;

#[api]
pub fn not_an_impl_block() {
    // This should fail because #[api] must be on an impl block
}

fn main() {}
