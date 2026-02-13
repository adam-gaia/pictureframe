use leptos::{prelude::*, task::spawn_local};
use pictureframe_common::{Client, Next, Photo};
use std::time::Duration;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

/// Metadata overlay that fades out after a few seconds
#[component]
fn PhotoOverlay(photo: Photo, #[prop(into)] visible: Signal<bool>) -> impl IntoView {
    let opacity = move || if visible.get() { "1" } else { "0" };

    view! {
        <div style:opacity=opacity style="
            position: absolute;
            bottom: 0;
            left: 0;
            right: 0;
            padding: 2rem;
            background: linear-gradient(transparent, rgba(0,0,0,0.7));
            color: white;
            font-family: system-ui, sans-serif;
            transition: opacity 1s ease-out;
        ">
            {photo.title.map(|t| view! { <h2 style="margin: 0 0 0.5rem 0; font-size: 1.5rem; text-shadow: 0 2px 4px rgba(0,0,0,0.5);">{t}</h2> })}
            <div style="display: flex; gap: 1.5rem; font-size: 0.9rem; opacity: 0.9;">
                {photo.artist.map(|a| view! { <span>{a}</span> })}
                {photo.date_taken.map(|d| view! { <span>{d.format("%B %d, %Y").to_string()}</span> })}
                {photo.copyright.map(|c| view! { <span>"© "{c}</span> })}
            </div>
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    // Client uses relative URLs - works when served from same origin
    let client = Client::new("");

    // Current photo state
    let (current, set_current) = signal(None::<Next>);
    let (overlay_visible, set_overlay_visible) = signal(true);

    // Trigger signal to request next photo fetch
    let (fetch_trigger, set_fetch_trigger) = signal(0u32);

    // Effect that fetches the next photo whenever fetch_trigger changes
    Effect::new(move |_| {
        // Subscribe to the trigger
        let _ = fetch_trigger.get();

        let client = client.clone();
        spawn_local(async move {
            match client.get_next().await {
                Ok(next) => {
                    let interval_secs = next.interval.seconds() as u64;
                    set_current.set(Some(next));

                    // Show overlay on image change
                    set_overlay_visible.set(true);
                    // Hide overlay after 5 seconds
                    set_timeout(
                        move || set_overlay_visible.set(false),
                        Duration::from_secs(5),
                    );

                    // Schedule next fetch after interval
                    set_timeout(
                        move || set_fetch_trigger.update(|n| *n = n.wrapping_add(1)),
                        Duration::from_secs(interval_secs),
                    );
                }
                Err(e) => {
                    log::error!("Failed to fetch image: {:?}", e);
                    // Retry after 30 seconds on error
                    set_timeout(
                        move || set_fetch_trigger.update(|n| *n = n.wrapping_add(1)),
                        Duration::from_secs(30),
                    );
                }
            }
        });
    });

    view! {
        <div style="
            width: 100vw;
            height: 100vh;
            margin: 0;
            padding: 0;
            overflow: hidden;
            background: black;
            position: relative;
        ">
            {move || current.get().map(|next| {
                let photo = next.photo.clone();
                let url = next.photo.url.clone();
                view! {
                    <img
                        src=url
                        style="
                            width: 100%;
                            height: 100%;
                            object-fit: contain;
                            display: block;
                        "
                    />
                    <PhotoOverlay photo=photo visible=overlay_visible />
                }
            })}
        </div>
    }
}
