use common::Client;
use leptos::{prelude::*, task::spawn_local};
use std::time::Duration;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let client = Client::new("localhost:3000"); // TODO

    // Configure polling interval (in seconds)
    let interval_secs = 5u64;

    // Store the current image URL
    let (image_url, set_image_url) = signal(None::<String>);

    // Fetch initial image on mount
    Effect::new(move |_| {
        let client = client.clone();
        spawn_local(async move {
            match client.get_next_image().await {
                Ok(url) => set_image_url.set(Some(url)),
                Err(e) => log::error!("Failed to fetch initial image: {:?}", e),
            }
        });
    });

    // Set up polling for new images
    let client_for_interval = client.clone();
    set_interval(
        move || {
            let client = client_for_interval.clone();
            spawn_local(async move {
                match client.get_next_image().await {
                    Ok(url) => set_image_url.set(Some(url)),
                    Err(e) => log::error!("Failed to fetch image: {:?}", e),
                }
            });
        },
        Duration::from_secs(interval_secs),
    );

    view! {
        <div style="
            width: 100vw;
            height: 100vh;
            margin: 0;
            padding: 0;
            overflow: hidden;
        ">
            {move || image_url.get().map(|url| view! {
                <img
                    src=url
                    style="
                        width: 100%;
                        height: 100%;
                        object-fit: cover;
                        display: block;
                    "
                />
            })}
        </div>
    }
}
