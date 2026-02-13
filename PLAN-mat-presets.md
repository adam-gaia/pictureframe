# Plan: Mat Presets Feature

## Overview

Add configurable mat (picture frame border) styles to the photo frame application. Each photo can have a mat preset assigned, which controls the color, size, and optional shadow around the image display. Presets are hardcoded in the application (not CRUD-managed), but users can select which preset to use for each photo via the admin panel.

## Current State

- **Viewer** (`crates/frontend-viewer/src/main.rs`): Has a hardcoded mat style:
  - Background: `#f5f2eb` (cream/off-white)
  - Padding: `4vmin`
  - No shadow
- **Database**: `photo` table has no mat-related columns
- **API**: `/api/next` returns `Next { photo, interval }` where `Photo` has no mat info
- **Admin**: `PhotoCard` component shows photo thumbnail with delete button, no mat selection

## Design Decisions

### 1. Mat Preset Storage

**Approach**: Store preset name as `TEXT NOT NULL DEFAULT 'classic'` on the `photo` table.

Rationale:
- Simple string column allows adding new presets without migrations
- Default ensures existing photos work without modification
- NOT NULL with default is safer than nullable (always has a value)
- String-based (not enum) for flexibility when adding new presets

### 2. Mat Preset Definition

Define presets in `crates/common/src/lib.rs` with these properties:
- `name: String` - preset identifier (e.g., "classic", "modern", "gallery")
- `background_color: String` - CSS color value
- `padding: String` - CSS padding value (e.g., "4vmin", "2rem")
- `shadow: Option<String>` - Optional CSS box-shadow value
- `inner_border: Option<String>` - Optional inner border (some frames have multiple layers)

### 3. API Changes

The `Photo` struct in `common` will include `mat_preset: String`. The `Next` response already includes the full `Photo`, so no structural change needed there.

For the viewer, we'll also add a `MatPreset` struct that contains the actual style values. The server will resolve the preset name to its style values and include them in the response. This keeps the viewer simple (no preset lookup logic needed client-side).

### 4. Preset List (Initial)

| Name | Background | Padding | Shadow | Description |
|------|------------|---------|--------|-------------|
| `classic` | `#f5f2eb` | `4vmin` | none | Current default, cream/off-white |
| `modern` | `#ffffff` | `3vmin` | `0 4px 20px rgba(0,0,0,0.15)` | Clean white with subtle shadow |
| `gallery` | `#2c2c2c` | `5vmin` | none | Dark gray, museum-style |
| `minimal` | `#f8f8f8` | `2vmin` | none | Light gray, thin border |
| `rich` | `#3a2a1a` | `4vmin` | `inset 0 0 30px rgba(0,0,0,0.3)` | Dark brown with inner shadow |
| `none` | `transparent` | `0` | none | No mat, full bleed |

## Implementation Steps

### Step 1: Database Migration

Create `migrations/005_add_mat_preset.sql`:

```sql
-- Add mat_preset column to photo table
ALTER TABLE photo ADD COLUMN mat_preset TEXT NOT NULL DEFAULT 'classic';
```

### Step 2: Update Database Model

In `src/models.rs`, add field to `DbPhoto`:

```rust
pub struct DbPhoto {
    // ... existing fields ...
    pub mat_preset: String,
}
```

### Step 3: Define Mat Types in Common Crate

In `crates/common/src/lib.rs`, add:

```rust
/// Visual style configuration for a mat (picture frame border)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatStyle {
    pub name: String,
    pub background_color: String,
    pub padding: String,
    pub shadow: Option<String>,
    pub inner_border: Option<String>,
}

impl MatStyle {
    /// Get the mat style for a given preset name
    pub fn from_preset(name: &str) -> Self {
        match name {
            "modern" => Self {
                name: "modern".into(),
                background_color: "#ffffff".into(),
                padding: "3vmin".into(),
                shadow: Some("0 4px 20px rgba(0,0,0,0.15)".into()),
                inner_border: None,
            },
            "gallery" => Self {
                name: "gallery".into(),
                background_color: "#2c2c2c".into(),
                padding: "5vmin".into(),
                shadow: None,
                inner_border: None,
            },
            "minimal" => Self {
                name: "minimal".into(),
                background_color: "#f8f8f8".into(),
                padding: "2vmin".into(),
                shadow: None,
                inner_border: None,
            },
            "rich" => Self {
                name: "rich".into(),
                background_color: "#3a2a1a".into(),
                padding: "4vmin".into(),
                shadow: Some("inset 0 0 30px rgba(0,0,0,0.3)".into()),
                inner_border: None,
            },
            "none" => Self {
                name: "none".into(),
                background_color: "transparent".into(),
                padding: "0".into(),
                shadow: None,
                inner_border: None,
            },
            // Default: classic
            _ => Self {
                name: "classic".into(),
                background_color: "#f5f2eb".into(),
                padding: "4vmin".into(),
                shadow: None,
                inner_border: None,
            },
        }
    }

    /// Get list of all available preset names
    pub fn preset_names() -> &'static [&'static str] {
        &["classic", "modern", "gallery", "minimal", "rich", "none"]
    }
}
```

### Step 4: Update Photo Struct

In `crates/common/src/lib.rs`, update `Photo`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub id: PhotoID,
    pub url: String,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub date_taken: Option<NaiveDateTime>,
    pub mat_preset: String,  // NEW
}
```

### Step 5: Update Next Response

In `crates/common/src/lib.rs`, update `Next`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Next {
    pub photo: Photo,
    pub interval: Interval,
    pub mat_style: MatStyle,  // NEW - resolved style for the preset
}
```

### Step 6: Update Server Conversion

In `src/app.rs`, update `db_photo_to_photo`:

```rust
fn db_photo_to_photo(input: &DbPhoto) -> Photo {
    Photo {
        id: PhotoID::from(input.id),
        url: format!("/api/images/{}", input.id),
        title: input.title.clone(),
        notes: input.notes.clone(),
        artist: input.artist.clone(),
        copyright: input.copyright.clone(),
        date_taken: input.date_taken,
        mat_preset: input.mat_preset.clone(),
    }
}
```

### Step 7: Update `/api/next` Handler

In `src/app.rs`, modify `get_next_photo`:

```rust
let photo = db_photo_to_photo(db_photo);
let mat_style = MatStyle::from_preset(&photo.mat_preset);
APIResult::Ok(Next { photo, interval, mat_style })
```

### Step 8: Add Update Photo Mat Endpoint

Add `mat_preset` to `UpdatePhotoRequest` in `crates/common/src/lib.rs`:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePhotoRequest {
    pub title: Option<Update<String>>,
    pub artist: Option<Update<String>>,
    pub copyright: Option<Update<String>>,
    pub date_taken: Option<Update<NaiveDateTime>>,
    pub mat_preset: Option<String>,  // NEW - just a string, no Update wrapper needed
}
```

Update the `update_photo` handler in `src/app.rs` to handle `mat_preset`:

```rust
// Update mat_preset if provided
if let Some(preset) = &req.mat_preset {
    // Validate preset exists
    if !MatStyle::preset_names().contains(&preset.as_str()) {
        return APIResult::InternalError(format!("Unknown mat preset: {}", preset));
    }
    if let Err(e) = sqlx::query(
        "UPDATE photo SET mat_preset = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(preset)
    .bind(id)
    .execute(&self.pool)
    .await
    {
        return APIResult::InternalError(format!("Failed to update photo: {}", e));
    }
}
```

### Step 9: Update Viewer Frontend

In `crates/frontend-viewer/src/main.rs`, use the mat style from the response:

```rust
// In the App component view
{move || current.get().map(|next| {
    let photo = next.photo.clone();
    let url = next.photo.url.clone();
    let mat = next.mat_style.clone();

    // Build dynamic style for outer container
    let outer_style = format!(
        "width: 100vw; height: 100vh; box-sizing: border-box; padding: {}; overflow: hidden; background: {};{}",
        mat.padding,
        mat.background_color,
        mat.shadow.as_ref().map(|s| format!(" box-shadow: {};", s)).unwrap_or_default()
    );

    view! {
        <div style=outer_style>
            <div style="width: 100%; height: 100%; position: relative; overflow: hidden;">
                <img
                    src=url
                    style="width: 100%; height: 100%; object-fit: cover; display: block;"
                />
                <PhotoOverlay photo=photo visible=overlay_visible />
            </div>
        </div>
    }
})}
```

### Step 10: Update Admin Panel - Photo Card

In `crates/frontend-admin/src/main.rs`, add mat preset selector to `PhotoCard`:

```rust
#[component]
fn PhotoCard<F>(photo: Photo, client: Client, on_refresh: F) -> impl IntoView
where
    F: Fn() + Clone + Send + 'static,
{
    let photo_id = photo.id;
    let current_preset = photo.mat_preset.clone();
    let (deleting, set_deleting) = signal(false);
    let (updating_mat, set_updating_mat) = signal(false);

    // ... existing delete handler ...

    let handle_mat_change = {
        let client = client.clone();
        let on_refresh = on_refresh.clone();
        move |preset: String| {
            if updating_mat.get() { return; }
            set_updating_mat.set(true);
            let client = client.clone();
            let on_refresh = on_refresh.clone();
            spawn_local(async move {
                let updates = UpdatePhotoRequest {
                    title: None,
                    artist: None,
                    copyright: None,
                    date_taken: None,
                    mat_preset: Some(preset),
                };
                match client.update_photo(photo_id, &updates).await {
                    Ok(_) => on_refresh(),
                    Err(e) => log::error!("Failed to update mat preset: {:?}", e),
                }
                set_updating_mat.set(false);
            });
        }
    };

    view! {
        <div style="border: 1px solid #e0e0e0; border-radius: 8px; overflow: hidden; background: white;">
            <img src=photo.url.clone() style="width: 100%; height: 150px; object-fit: cover;" loading="lazy" />
            <div style="padding: 0.75rem;">
                <div style="font-weight: 500; margin-bottom: 0.25rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    {photo.title.clone().unwrap_or_else(|| format!("Photo {}", photo.id.0))}
                </div>
                {photo.artist.map(|a| view! { <div style="font-size: 0.85rem; color: #666;">{a}</div> })}

                // Mat preset selector
                <div style="margin-top: 0.5rem;">
                    <label style="font-size: 0.8rem; color: #666; display: block; margin-bottom: 0.25rem;">"Mat Style"</label>
                    <select
                        style="width: 100%; padding: 0.25rem; border: 1px solid #ccc; border-radius: 4px; font-size: 0.85rem;"
                        on:change={
                            let handle = handle_mat_change.clone();
                            move |ev| handle(event_target_value(&ev))
                        }
                        disabled=move || updating_mat.get()
                    >
                        {["classic", "modern", "gallery", "minimal", "rich", "none"].into_iter().map(|preset| {
                            view! {
                                <option value=preset selected=current_preset == preset>
                                    {preset}
                                </option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>
                </div>

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
```

### Step 11: Add Mat Preset Names API Endpoint (Optional)

Add an endpoint to get available preset names for dynamic UI generation:

```rust
#[api_handler(method = "GET", path = "/api/mat-presets")]
pub async fn get_mat_presets(&self) -> APIResult<Vec<MatStyle>> {
    let presets = MatStyle::preset_names()
        .iter()
        .map(|name| MatStyle::from_preset(name))
        .collect();
    APIResult::Ok(presets)
}
```

This allows the admin panel to dynamically show preset options with previews if desired later.

## File Changes Summary

| File | Change |
|------|--------|
| `migrations/005_add_mat_preset.sql` | NEW - Add mat_preset column |
| `src/models.rs` | Add `mat_preset: String` to `DbPhoto` |
| `crates/common/src/lib.rs` | Add `MatStyle` struct, update `Photo`, `Next`, `UpdatePhotoRequest` |
| `src/app.rs` | Update `db_photo_to_photo`, `get_next_photo`, `update_photo` handlers |
| `crates/frontend-viewer/src/main.rs` | Apply dynamic mat styles from response |
| `crates/frontend-admin/src/main.rs` | Add mat preset dropdown to `PhotoCard` |

## Testing Plan

1. **Migration**: Run `sqlx migrate run` and verify column exists with default value
2. **API**: Test `/api/next` returns `mat_style` object with correct values
3. **API**: Test `PUT /api/photos/{id}` with `mat_preset` updates correctly
4. **Viewer**: Verify each preset renders correctly with expected colors/padding/shadows
5. **Admin**: Verify dropdown shows all presets and updates persist

## Future Enhancements (Out of Scope)

- Per-album mat preset (override individual photo settings)
- Global default mat preset in settings
- Custom mat presets (user-defined colors/sizes)
- Mat preview thumbnails in admin panel
- Transition animations between different mat styles
