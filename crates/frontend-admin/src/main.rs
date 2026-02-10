use leptos::prelude::*;

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    view! {
        <main>
            <h1>"Hello from Admin"</h1>
        </main>
    }
}
