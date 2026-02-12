use common::{
    Album, AlbumID, Client, CreateAlbumRequest, Photo, RotationSettings, Update,
    UpdateSettingsRequest,
};
use leptos::{prelude::*, task::spawn_local};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FormData, HtmlInputElement, Request, RequestInit, Response};

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Photos,
    Albums,
    Settings,
}

#[component]
fn App() -> impl IntoView {
    let client = Client::new("");

    let (active_tab, set_active_tab) = signal(Tab::Photos);

    // Shared state for photos and albums
    let (photos, set_photos) = signal(Vec::<Photo>::new());
    let (albums, set_albums) = signal(Vec::<Album>::new());
    let (settings, set_settings) = signal(None::<RotationSettings>);

    // Refresh functions
    let refresh_photos = {
        let client = client.clone();
        move || {
            let client = client.clone();
            spawn_local(async move {
                match client.get_photos().await {
                    Ok(p) => set_photos.set(p),
                    Err(e) => log::error!("Failed to fetch photos: {:?}", e),
                }
            });
        }
    };

    let refresh_albums = {
        let client = client.clone();
        move || {
            let client = client.clone();
            spawn_local(async move {
                match client.get_albums().await {
                    Ok(a) => set_albums.set(a),
                    Err(e) => log::error!("Failed to fetch albums: {:?}", e),
                }
            });
        }
    };

    let refresh_settings = {
        let client = client.clone();
        move || {
            let client = client.clone();
            spawn_local(async move {
                match client.get_settings().await {
                    Ok(s) => set_settings.set(Some(s)),
                    Err(e) => log::error!("Failed to fetch settings: {:?}", e),
                }
            });
        }
    };

    // Initial load
    Effect::new({
        let refresh_photos = refresh_photos.clone();
        let refresh_albums = refresh_albums.clone();
        let refresh_settings = refresh_settings.clone();
        move |_| {
            refresh_photos();
            refresh_albums();
            refresh_settings();
        }
    });

    view! {
        <div style="font-family: system-ui, sans-serif; max-width: 1200px; margin: 0 auto; padding: 1rem;">
            <h1 style="margin-bottom: 1rem;">"Photo Frame Admin"</h1>

            // Tab navigation
            <nav style="display: flex; gap: 0.5rem; margin-bottom: 1.5rem; border-bottom: 2px solid #e0e0e0; padding-bottom: 0.5rem;">
                <TabButton tab=Tab::Photos active=active_tab set_active=set_active_tab label="Photos" />
                <TabButton tab=Tab::Albums active=active_tab set_active=set_active_tab label="Albums" />
                <TabButton tab=Tab::Settings active=active_tab set_active=set_active_tab label="Settings" />
            </nav>

            // Tab content
            {move || match active_tab.get() {
                Tab::Photos => view! {
                    <PhotosTab
                        photos=photos
                        client=client.clone()
                        on_refresh=refresh_photos.clone()
                    />
                }.into_any(),
                Tab::Albums => view! {
                    <AlbumsTab
                        albums=albums
                        photos=photos
                        client=client.clone()
                        on_refresh_albums=refresh_albums.clone()
                    />
                }.into_any(),
                Tab::Settings => view! {
                    <SettingsTab
                        settings=settings
                        albums=albums
                        client=client.clone()
                        on_refresh=refresh_settings.clone()
                    />
                }.into_any(),
            }}
        </div>
    }
}

#[component]
fn TabButton(
    tab: Tab,
    active: ReadSignal<Tab>,
    set_active: WriteSignal<Tab>,
    label: &'static str,
) -> impl IntoView {
    let is_active = move || active.get() == tab;
    let style = move || {
        if is_active() {
            "padding: 0.5rem 1rem; border: none; background: #2196F3; color: white; cursor: pointer; border-radius: 4px 4px 0 0; font-size: 1rem;"
        } else {
            "padding: 0.5rem 1rem; border: none; background: #f0f0f0; color: #333; cursor: pointer; border-radius: 4px 4px 0 0; font-size: 1rem;"
        }
    };

    view! {
        <button style=style on:click=move |_| set_active.set(tab)>
            {label}
        </button>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Photos Tab
// ─────────────────────────────────────────────────────────────────────────────

#[component]
fn PhotosTab<F>(photos: ReadSignal<Vec<Photo>>, client: Client, on_refresh: F) -> impl IntoView
where
    F: Fn() + Clone + Send + 'static,
{
    let (upload_status, set_upload_status) = signal(None::<String>);
    let (upload_error, set_upload_error) = signal(None::<String>);
    let file_input_ref = NodeRef::<leptos::html::Input>::new();

    let handle_file_change = {
        let on_refresh = on_refresh.clone();
        move |_| {
            let Some(input) = file_input_ref.get() else {
                return;
            };
            let input: HtmlInputElement = input.into();
            let Some(files) = input.files() else {
                return;
            };

            let file_count = files.length();
            if file_count == 0 {
                return;
            }

            // Collect all files into a Vec
            let files: Vec<web_sys::File> = (0..file_count)
                .filter_map(|i| files.get(i))
                .collect();

            set_upload_status.set(Some(format!("Uploading 0/{}", file_count)));
            set_upload_error.set(None);
            let on_refresh = on_refresh.clone();

            spawn_local(async move {
                let mut errors: Vec<String> = Vec::new();
                let total = files.len();

                for (i, file) in files.into_iter().enumerate() {
                    set_upload_status.set(Some(format!("Uploading {}/{}...", i + 1, total)));

                    if let Err(e) = upload_photo(file).await {
                        log::error!("Upload failed: {}", e);
                        errors.push(e);
                    }
                }

                set_upload_status.set(None);

                if errors.is_empty() {
                    on_refresh();
                } else {
                    // Refresh to show any that succeeded
                    on_refresh();
                    // Show errors
                    let error_msg = if errors.len() == 1 {
                        errors[0].clone()
                    } else {
                        format!("{} uploads failed", errors.len())
                    };
                    set_upload_error.set(Some(error_msg));
                }

                // Clear the input so the same files can be selected again
                if let Some(input) = file_input_ref.get() {
                    let input: HtmlInputElement = input.into();
                    input.set_value("");
                }
            });
        }
    };

    let trigger_upload = move |_| {
        if let Some(input) = file_input_ref.get() {
            let input: HtmlInputElement = input.into();
            input.click();
        }
    };

    let is_uploading = move || upload_status.get().is_some();

    view! {
        <div>
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
                <h2 style="margin: 0;">"Photos (" {move || photos.get().len()} ")"</h2>
                <div style="display: flex; gap: 0.5rem;">
                    // Hidden file input (multiple files allowed)
                    <input
                        type="file"
                        accept="image/jpeg"
                        multiple=true
                        node_ref=file_input_ref
                        style="display: none;"
                        on:change=handle_file_change
                    />
                    // Upload button
                    <button
                        style="padding: 0.5rem 1rem; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer;"
                        on:click=trigger_upload
                        disabled=is_uploading
                    >
                        {move || upload_status.get().unwrap_or_else(|| "Upload Photos".to_string())}
                    </button>
                    // Refresh button
                    <button
                        style="padding: 0.5rem 1rem; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer;"
                        on:click={
                            let on_refresh = on_refresh.clone();
                            move |_| on_refresh()
                        }
                        disabled=is_uploading
                    >
                        "Refresh"
                    </button>
                </div>
            </div>

            // Upload error message
            {move || upload_error.get().map(|err| view! {
                <div style="background: #ffebee; color: #c62828; padding: 0.75rem; border-radius: 4px; margin-bottom: 1rem;">
                    {err}
                </div>
            })}

            <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 1rem;">
                {move || photos.get().into_iter().map(|photo| {
                    let client = client.clone();
                    let on_refresh = on_refresh.clone();
                    view! { <PhotoCard photo=photo client=client on_delete=on_refresh /> }
                }).collect::<Vec<_>>()}
            </div>

            {move || if photos.get().is_empty() {
                Some(view! {
                    <p style="color: #666; text-align: center; padding: 2rem;">
                        "No photos yet. Click \"Upload Photo\" to add your first photo."
                    </p>
                })
            } else {
                None
            }}
        </div>
    }
}

/// Upload a photo file to the server via multipart form data.
async fn upload_photo(file: web_sys::File) -> Result<(), String> {
    let form_data = FormData::new().map_err(|e| format!("Failed to create FormData: {:?}", e))?;
    form_data
        .append_with_blob_and_filename("file", &file, &file.name())
        .map_err(|e| format!("Failed to append file: {:?}", e))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&form_data);

    let request = Request::new_with_str_and_init("/api/photos", &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let window = web_sys::window().ok_or("No window object")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object")?;

    if !resp.ok() {
        let status = resp.status();
        let body = JsFuture::from(resp.text().map_err(|_| "Failed to get response text")?)
            .await
            .map_err(|_| "Failed to read response body")?
            .as_string()
            .unwrap_or_default();
        return Err(format!("Server error ({}): {}", status, body));
    }

    Ok(())
}

#[component]
fn PhotoCard<F>(photo: Photo, client: Client, on_delete: F) -> impl IntoView
where
    F: Fn() + Clone + Send + 'static,
{
    let photo_id = photo.id;
    let (deleting, set_deleting) = signal(false);

    let handle_delete = {
        let client = client.clone();
        move |_| {
            if deleting.get() {
                return;
            }
            set_deleting.set(true);
            let client = client.clone();
            let on_delete = on_delete.clone();
            spawn_local(async move {
                match client.delete_photo(photo_id).await {
                    Ok(_) => on_delete(),
                    Err(e) => log::error!("Failed to delete photo: {:?}", e),
                }
                set_deleting.set(false);
            });
        }
    };

    view! {
        <div style="border: 1px solid #e0e0e0; border-radius: 8px; overflow: hidden; background: white;">
            <img
                src=photo.url.clone()
                style="width: 100%; height: 150px; object-fit: cover;"
                loading="lazy"
            />
            <div style="padding: 0.75rem;">
                <div style="font-weight: 500; margin-bottom: 0.25rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    {photo.title.clone().unwrap_or_else(|| format!("Photo {}", photo.id.0))}
                </div>
                {photo.artist.map(|a| view! {
                    <div style="font-size: 0.85rem; color: #666;">{a}</div>
                })}
                <button
                    style="margin-top: 0.5rem; padding: 0.25rem 0.5rem; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.8rem;"
                    on:click=handle_delete
                    disabled=move || deleting.get()
                >
                    {move || if deleting.get() { "Deleting..." } else { "Delete" }}
                </button>
            </div>
        </div>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Albums Tab
// ─────────────────────────────────────────────────────────────────────────────

#[component]
fn AlbumsTab<F>(
    albums: ReadSignal<Vec<Album>>,
    photos: ReadSignal<Vec<Photo>>,
    client: Client,
    on_refresh_albums: F,
) -> impl IntoView
where
    F: Fn() + Clone + Send + 'static,
{
    let (new_album_name, set_new_album_name) = signal(String::new());
    let (creating, set_creating) = signal(false);
    let (selected_album, set_selected_album) = signal(None::<AlbumID>);

    let handle_create = {
        let client = client.clone();
        let on_refresh = on_refresh_albums.clone();
        move |_| {
            let name = new_album_name.get();
            if name.trim().is_empty() || creating.get() {
                return;
            }
            set_creating.set(true);
            let client = client.clone();
            let on_refresh = on_refresh.clone();
            spawn_local(async move {
                match client
                    .create_album(&CreateAlbumRequest {
                        name,
                        notes: None,
                    })
                    .await
                {
                    Ok(_) => {
                        set_new_album_name.set(String::new());
                        on_refresh();
                    }
                    Err(e) => log::error!("Failed to create album: {:?}", e),
                }
                set_creating.set(false);
            });
        }
    };

    view! {
        <div>
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
                <h2 style="margin: 0;">"Albums (" {move || albums.get().len()} ")"</h2>
            </div>

            // Create album form
            <div style="display: flex; gap: 0.5rem; margin-bottom: 1.5rem;">
                <input
                    type="text"
                    placeholder="New album name..."
                    style="flex: 1; padding: 0.5rem; border: 1px solid #ccc; border-radius: 4px;"
                    prop:value=move || new_album_name.get()
                    on:input=move |ev| set_new_album_name.set(event_target_value(&ev))
                />
                <button
                    style="padding: 0.5rem 1rem; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer;"
                    on:click=handle_create
                    disabled=move || creating.get() || new_album_name.get().trim().is_empty()
                >
                    {move || if creating.get() { "Creating..." } else { "Create Album" }}
                </button>
            </div>

            // Album list
            <div style="display: flex; gap: 1rem;">
                // Album sidebar
                <div style="width: 250px; border-right: 1px solid #e0e0e0; padding-right: 1rem;">
                    {move || albums.get().into_iter().map(|album| {
                        let album_id = album.id;
                        let is_selected = move || selected_album.get() == Some(album_id);
                        let style = move || if is_selected() {
                            "padding: 0.75rem; background: #e3f2fd; border-radius: 4px; cursor: pointer; margin-bottom: 0.5rem;"
                        } else {
                            "padding: 0.75rem; background: #f5f5f5; border-radius: 4px; cursor: pointer; margin-bottom: 0.5rem;"
                        };
                        view! {
                            <div style=style on:click=move |_| set_selected_album.set(Some(album_id))>
                                <div style="font-weight: 500;">{album.name.clone()}</div>
                                <div style="font-size: 0.85rem; color: #666;">
                                    {album.photos.len()} " photos"
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                // Album detail
                <div style="flex: 1;">
                    {move || {
                        let sel_id = selected_album.get();
                        let album = sel_id.and_then(|id| albums.get().into_iter().find(|a| a.id.0 == id.0));
                        match album {
                            Some(album) => view! {
                                <AlbumDetail
                                    album=album
                                    photos=photos
                                    client=client.clone()
                                    on_refresh=on_refresh_albums.clone()
                                />
                            }.into_any(),
                            None => view! {
                                <p style="color: #666; text-align: center; padding: 2rem;">
                                    "Select an album to manage its photos"
                                </p>
                            }.into_any(),
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
fn AlbumDetail<F>(
    album: Album,
    photos: ReadSignal<Vec<Photo>>,
    client: Client,
    on_refresh: F,
) -> impl IntoView
where
    F: Fn() + Clone + Send + 'static,
{
    let album_id = album.id;
    let album_photo_ids: Vec<i32> = album.photos.iter().map(|p| p.0).collect();
    let (deleting, set_deleting) = signal(false);

    let handle_delete_album = {
        let client = client.clone();
        let on_refresh = on_refresh.clone();
        move |_| {
            if deleting.get() {
                return;
            }
            set_deleting.set(true);
            let client = client.clone();
            let on_refresh = on_refresh.clone();
            spawn_local(async move {
                match client.delete_album(album_id).await {
                    Ok(_) => on_refresh(),
                    Err(e) => log::error!("Failed to delete album: {:?}", e),
                }
                set_deleting.set(false);
            });
        }
    };

    view! {
        <div>
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
                <h3 style="margin: 0;">{album.name}</h3>
                <button
                    style="padding: 0.25rem 0.5rem; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.85rem;"
                    on:click=handle_delete_album
                    disabled=move || deleting.get()
                >
                    {move || if deleting.get() { "Deleting..." } else { "Delete Album" }}
                </button>
            </div>

            <h4 style="margin: 1rem 0 0.5rem 0;">"Photos in Album"</h4>
            <div style="display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 1.5rem;">
                {
                    let album_photo_ids = album_photo_ids.clone();
                    let client = client.clone();
                    let on_refresh = on_refresh.clone();
                    move || {
                        let album_ids = album_photo_ids.clone();
                        photos.get().into_iter()
                            .filter(|p| album_ids.contains(&p.id.0))
                            .map(|photo| {
                                let photo_id = photo.id;
                                let client = client.clone();
                                let on_refresh = on_refresh.clone();
                                view! {
                                    <div style="position: relative; width: 80px; height: 80px;">
                                        <img
                                            src=photo.url.clone()
                                            style="width: 100%; height: 100%; object-fit: cover; border-radius: 4px;"
                                        />
                                        <button
                                            style="position: absolute; top: 2px; right: 2px; width: 20px; height: 20px; background: rgba(244,67,54,0.9); color: white; border: none; border-radius: 50%; cursor: pointer; font-size: 12px; line-height: 1;"
                                            on:click={
                                                let client = client.clone();
                                                let on_refresh = on_refresh.clone();
                                                move |_| {
                                                    let client = client.clone();
                                                    let on_refresh = on_refresh.clone();
                                                    spawn_local(async move {
                                                        if let Err(e) = client.remove_photo_from_album(album_id, photo_id).await {
                                                            log::error!("Failed to remove photo: {:?}", e);
                                                        }
                                                        on_refresh();
                                                    });
                                                }
                                            }
                                        >
                                            "×"
                                        </button>
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                    }
                }
            </div>

            <h4 style="margin: 1rem 0 0.5rem 0;">"Add Photos"</h4>
            <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
                {
                    let album_photo_ids = album_photo_ids.clone();
                    let client = client.clone();
                    let on_refresh = on_refresh.clone();
                    move || {
                        let album_ids = album_photo_ids.clone();
                        photos.get().into_iter()
                            .filter(|p| !album_ids.contains(&p.id.0))
                            .map(|photo| {
                                let photo_id = photo.id;
                                let client = client.clone();
                                let on_refresh = on_refresh.clone();
                                view! {
                                    <div
                                        style="width: 80px; height: 80px; cursor: pointer; opacity: 0.7; transition: opacity 0.2s;"
                                        on:click={
                                            let client = client.clone();
                                            let on_refresh = on_refresh.clone();
                                            move |_| {
                                                let client = client.clone();
                                                let on_refresh = on_refresh.clone();
                                                spawn_local(async move {
                                                    if let Err(e) = client.add_photo_to_album(album_id, photo_id).await {
                                                        log::error!("Failed to add photo: {:?}", e);
                                                    }
                                                    on_refresh();
                                                });
                                            }
                                        }
                                    >
                                        <img
                                            src=photo.url.clone()
                                            style="width: 100%; height: 100%; object-fit: cover; border-radius: 4px; border: 2px dashed #ccc;"
                                        />
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                    }
                }
            </div>
        </div>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings Tab
// ─────────────────────────────────────────────────────────────────────────────

#[component]
fn SettingsTab<F>(
    settings: ReadSignal<Option<RotationSettings>>,
    albums: ReadSignal<Vec<Album>>,
    client: Client,
    on_refresh: F,
) -> impl IntoView
where
    F: Fn() + Clone + Send + 'static,
{
    let (saving, set_saving) = signal(false);

    let handle_set_album = {
        let client = client.clone();
        let on_refresh = on_refresh.clone();
        move |album_id: Option<AlbumID>| {
            if saving.get() {
                return;
            }
            set_saving.set(true);
            let client = client.clone();
            let on_refresh = on_refresh.clone();
            spawn_local(async move {
                let update = UpdateSettingsRequest {
                    current_album_id: Some(match album_id {
                        Some(id) => Update::Set(id),
                        None => Update::Remove,
                    }),
                    interval_seconds: None,
                };
                match client.update_settings(&update).await {
                    Ok(_) => on_refresh(),
                    Err(e) => log::error!("Failed to update settings: {:?}", e),
                }
                set_saving.set(false);
            });
        }
    };

    let handle_set_interval = {
        let client = client.clone();
        let on_refresh = on_refresh.clone();
        move |seconds: i32| {
            if saving.get() {
                return;
            }
            set_saving.set(true);
            let client = client.clone();
            let on_refresh = on_refresh.clone();
            spawn_local(async move {
                let update = UpdateSettingsRequest {
                    current_album_id: None,
                    interval_seconds: Some(seconds),
                };
                match client.update_settings(&update).await {
                    Ok(_) => on_refresh(),
                    Err(e) => log::error!("Failed to update settings: {:?}", e),
                }
                set_saving.set(false);
            });
        }
    };

    view! {
        <div>
            <h2 style="margin-bottom: 1.5rem;">"Settings"</h2>

            {move || settings.get().map(|s| {
                let current_album_id = s.current_album.as_ref().map(|c| c.album.0);
                let interval = s.interval.seconds();

                view! {
                    <div style="max-width: 500px;">
                        // Current album
                        <div style="margin-bottom: 1.5rem;">
                            <label style="display: block; font-weight: 500; margin-bottom: 0.5rem;">
                                "Current Album"
                            </label>
                            <select
                                style="width: 100%; padding: 0.5rem; border: 1px solid #ccc; border-radius: 4px; font-size: 1rem;"
                                on:change={
                                    let handle = handle_set_album.clone();
                                    move |ev| {
                                        let value = event_target_value(&ev);
                                        let album_id = if value == "none" {
                                            None
                                        } else {
                                            value.parse::<i32>().ok().map(AlbumID)
                                        };
                                        handle(album_id);
                                    }
                                }
                                disabled=move || saving.get()
                            >
                                <option value="none" selected=current_album_id.is_none()>
                                    "(All photos - no album selected)"
                                </option>
                                {albums.get().into_iter().map(|album| {
                                    let is_selected = current_album_id == Some(album.id.0);
                                    view! {
                                        <option value=album.id.0.to_string() selected=is_selected>
                                            {album.name} " (" {album.photos.len()} " photos)"
                                        </option>
                                    }
                                }).collect::<Vec<_>>()}
                            </select>
                            <p style="font-size: 0.85rem; color: #666; margin-top: 0.25rem;">
                                "When no album is selected, photos are pulled from the entire library."
                            </p>
                        </div>

                        // Interval
                        <div style="margin-bottom: 1.5rem;">
                            <label style="display: block; font-weight: 500; margin-bottom: 0.5rem;">
                                "Display Duration: " {interval} " seconds"
                            </label>
                            <div style="display: flex; gap: 0.5rem; flex-wrap: wrap;">
                                {[30, 60, 120, 180, 300, 600].into_iter().map(|secs| {
                                    let is_current = interval == secs as u32;
                                    let label = if secs < 60 {
                                        format!("{}s", secs)
                                    } else {
                                        format!("{}m", secs / 60)
                                    };
                                    let style = if is_current {
                                        "padding: 0.5rem 1rem; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer;"
                                    } else {
                                        "padding: 0.5rem 1rem; background: #f0f0f0; color: #333; border: none; border-radius: 4px; cursor: pointer;"
                                    };
                                    let handle = handle_set_interval.clone();
                                    view! {
                                        <button
                                            style=style
                                            on:click=move |_| handle(secs)
                                            disabled=move || saving.get()
                                        >
                                            {label}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>

                        {move || if saving.get() {
                            Some(view! { <p style="color: #2196F3;">"Saving..."</p> })
                        } else {
                            None
                        }}
                    </div>
                }
            })}
        </div>
    }
}
