# Plan: Mat Presets Phase 2 - Tests & Preview Functionality

## Overview

This plan covers two areas:
1. **Tests** - New tests to validate the mat presets feature implemented in Phase 1
2. **Mat Preview** - Visual preview functionality in the admin panel

---

## Part A: Test Plan

### Current Test Infrastructure

The codebase has a robust test suite:
- `tests/api.rs` - API endpoint integration tests (65+ tests)
- `tests/client.rs` - HTTP client tests
- `tests/e2e.rs` - End-to-end tests with real images
- `src/test_helpers.rs` - Shared test utilities (in-memory SQLite, seeders)

Tests use:
- `#[tokio::test]` for async tests
- In-memory SQLite databases for isolation
- `tower::ServiceExt::oneshot()` for direct router testing

### New Tests Required

#### A1. Unit Tests for MatStyle (in common crate)

Add tests in `crates/common/src/lib.rs` or new `crates/common/src/tests.rs`:

```rust
#[cfg(test)]
mod mat_style_tests {
    use super::*;

    #[test]
    fn test_from_preset_classic() {
        let style = MatStyle::from_preset("classic");
        assert_eq!(style.name, "classic");
        assert_eq!(style.background_color, "#f5f2eb");
        assert_eq!(style.padding, "4vmin");
        assert!(style.shadow.is_none());
    }

    #[test]
    fn test_from_preset_modern_has_shadow() {
        let style = MatStyle::from_preset("modern");
        assert_eq!(style.name, "modern");
        assert!(style.shadow.is_some());
    }

    #[test]
    fn test_from_preset_unknown_defaults_to_classic() {
        let style = MatStyle::from_preset("unknown_preset");
        assert_eq!(style.name, "classic");
    }

    #[test]
    fn test_preset_names_contains_all() {
        let names = MatStyle::preset_names();
        assert!(names.contains(&"classic"));
        assert!(names.contains(&"modern"));
        assert!(names.contains(&"gallery"));
        assert!(names.contains(&"minimal"));
        assert!(names.contains(&"rich"));
        assert!(names.contains(&"none"));
        assert_eq!(names.len(), 6);
    }

    #[test]
    fn test_all_presets_are_valid() {
        // Ensure from_preset works for all preset_names
        for name in MatStyle::preset_names() {
            let style = MatStyle::from_preset(name);
            assert_eq!(style.name, *name);
        }
    }
}
```

#### A2. API Integration Tests (in tests/api.rs)

Add new test section for mat presets:

```rust
// ─────────────────────────────────────────────────────────────────────────────
// Mat Presets
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_mat_presets_returns_all_presets() {
    let app = create_test_app().await;
    let router = create_test_router(app);

    let response = router
        .oneshot(Request::builder().uri("/api/mat-presets").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = get_body_json(response).await;
    let presets: Vec<serde_json::Value> = serde_json::from_value(body).unwrap();

    assert_eq!(presets.len(), 6);
    let names: Vec<&str> = presets.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"classic"));
    assert!(names.contains(&"modern"));
    assert!(names.contains(&"gallery"));
    assert!(names.contains(&"minimal"));
    assert!(names.contains(&"rich"));
    assert!(names.contains(&"none"));
}

#[tokio::test]
async fn test_photo_has_default_mat_preset() {
    let app = create_test_app().await;
    seed_photo(&app, 1, "test.jpg").await;
    let router = create_test_router(app);

    let response = router
        .oneshot(Request::builder().uri("/api/photos/1").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = get_body_json(response).await;
    assert_eq!(body["mat_preset"], "classic");
}

#[tokio::test]
async fn test_update_photo_mat_preset() {
    let app = create_test_app().await;
    seed_photo(&app, 1, "test.jpg").await;
    let router = create_test_router(app.clone());

    let update = json!({ "mat_preset": "gallery" });
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/photos/1")
                .header("Content-Type", "application/json")
                .body(Body::from(update.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify the update persisted
    let router = create_test_router(app);
    let response = router
        .oneshot(Request::builder().uri("/api/photos/1").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = get_body_json(response).await;
    assert_eq!(body["mat_preset"], "gallery");
}

#[tokio::test]
async fn test_update_photo_invalid_mat_preset_fails() {
    let app = create_test_app().await;
    seed_photo(&app, 1, "test.jpg").await;
    let router = create_test_router(app);

    let update = json!({ "mat_preset": "invalid_preset" });
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/photos/1")
                .header("Content-Type", "application/json")
                .body(Body::from(update.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = get_body_json(response).await;
    assert!(body["error"].as_str().unwrap().contains("Unknown mat preset"));
}

#[tokio::test]
async fn test_next_photo_includes_mat_style() {
    let app = create_test_app().await;
    seed_photo(&app, 1, "test.jpg").await;
    let router = create_test_router(app);

    let response = router
        .oneshot(Request::builder().uri("/api/next").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = get_body_json(response).await;

    // Check mat_style is present and has expected fields
    assert!(body["mat_style"].is_object());
    assert_eq!(body["mat_style"]["name"], "classic");
    assert_eq!(body["mat_style"]["background_color"], "#f5f2eb");
    assert_eq!(body["mat_style"]["padding"], "4vmin");
}

#[tokio::test]
async fn test_next_photo_mat_style_matches_photo_preset() {
    let app = create_test_app().await;
    seed_photo(&app, 1, "test.jpg").await;

    // Update photo to use "modern" preset
    sqlx::query("UPDATE photo SET mat_preset = 'modern' WHERE id = 1")
        .execute(app.pool())
        .await
        .unwrap();

    let router = create_test_router(app);
    let response = router
        .oneshot(Request::builder().uri("/api/next").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = get_body_json(response).await;
    assert_eq!(body["mat_style"]["name"], "modern");
    assert!(body["mat_style"]["shadow"].is_string()); // modern has shadow
}

#[tokio::test]
async fn test_all_mat_presets_have_required_fields() {
    let app = create_test_app().await;
    let router = create_test_router(app);

    let response = router
        .oneshot(Request::builder().uri("/api/mat-presets").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = get_body_json(response).await;
    let presets: Vec<serde_json::Value> = serde_json::from_value(body).unwrap();

    for preset in presets {
        assert!(preset["name"].is_string(), "name should be string");
        assert!(preset["background_color"].is_string(), "background_color should be string");
        assert!(preset["padding"].is_string(), "padding should be string");
        // shadow and inner_border can be null or string
    }
}
```

#### A3. Update Test Helper (seed_photo)

The `seed_photo` helper in `src/test_helpers.rs` needs to include `mat_preset`:

```rust
pub async fn seed_photo(app: &App, id: i32, hash: &str) {
    sqlx::query(
        r#"
        INSERT INTO photo (id, hash, fullsize_path, websize_path, thumbnail_path, mat_preset)
        VALUES (?, ?, 'full.jpg', 'web.jpg', 'thumb.jpg', 'classic')
        "#,
    )
    .bind(id)
    .bind(hash)
    .execute(app.pool())
    .await
    .unwrap();
}
```

### Test Implementation Steps

| Step | Description |
|------|-------------|
| A1 | Add MatStyle unit tests to common crate |
| A2 | Update seed_photo helper to include mat_preset |
| A3 | Add mat presets API tests to tests/api.rs |
| A4 | Run full test suite to verify no regressions |

---

## Part B: Mat Preview Functionality

### Goal

Add visual preview of mat styles in the admin panel so users can see what each preset looks like before selecting it.

### Design Options

#### Option 1: Inline Mini Preview (Recommended)
Show a small preview square next to each option in the dropdown, or replace dropdown with visual picker.

**Pros:** Immediate visual feedback, compact
**Cons:** Limited space for shadow effects

#### Option 2: Preview Panel
Show a larger preview panel that updates when hovering over options.

**Pros:** Larger preview area, can show shadows clearly
**Cons:** Takes more screen space

#### Option 3: Modal Picker
Open a modal with all presets shown as cards with previews.

**Pros:** Most space for previews, can show all at once
**Cons:** Extra click required, interrupts flow

### Recommended Approach: Visual Picker Cards

Replace the dropdown with a visual card picker showing small preview thumbnails of each mat style.

### UI Mockup

```
┌─────────────────────────────────────────┐
│ Mat Style                               │
├─────────────────────────────────────────┤
│ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐        │
│ │█████│ │█████│ │█████│ │█████│        │
│ │█   █│ │█   █│ │█   █│ │█   █│        │
│ │█████│ │█████│ │█████│ │█████│        │
│ └─────┘ └─────┘ └─────┘ └─────┘        │
│ classic  modern  gallery minimal        │
│                                         │
│ ┌─────┐ ┌─────┐                        │
│ │█████│ │     │                        │
│ │█   █│ │  █  │                        │
│ │█████│ │     │                        │
│ └─────┘ └─────┘                        │
│  rich    none                          │
└─────────────────────────────────────────┘
```

Each card shows:
- Background color of the mat
- A small "photo" rectangle in the center
- Shadow effect (where applicable)
- Preset name below
- Visual selection indicator (border/highlight) for current preset

### Implementation

#### B1. Create MatPreview Component

In `crates/frontend-admin/src/main.rs`, add a new component:

```rust
/// Visual preview of a mat style
#[component]
fn MatPreview(style: MatStyle, size: u32) -> impl IntoView {
    let outer_style = format!(
        "width: {}px; height: {}px; background: {}; display: flex; align-items: center; justify-content: center; box-sizing: border-box;{}",
        size,
        size,
        style.background_color,
        style.shadow.as_ref().map(|s| format!(" box-shadow: {};", s)).unwrap_or_default()
    );

    // Calculate inner "photo" size based on padding
    // For preview, use a fixed ratio
    let inner_size = (size as f32 * 0.6) as u32;
    let inner_style = format!(
        "width: {}px; height: {}px; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); border-radius: 2px;",
        inner_size,
        inner_size
    );

    view! {
        <div style=outer_style>
            <div style=inner_style></div>
        </div>
    }
}
```

#### B2. Create MatPresetPicker Component

```rust
/// Visual picker for mat presets
#[component]
fn MatPresetPicker<F>(
    current_preset: String,
    on_change: F,
    disabled: bool,
) -> impl IntoView
where
    F: Fn(String) + Clone + 'static,
{
    view! {
        <div style="margin-top: 0.5rem;">
            <label style="font-size: 0.8rem; color: #666; display: block; margin-bottom: 0.5rem;">
                "Mat Style"
            </label>
            <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
                {MatStyle::preset_names().iter().map(|preset_name| {
                    let preset = *preset_name;
                    let style = MatStyle::from_preset(preset);
                    let is_selected = current_preset == preset;
                    let on_change = on_change.clone();

                    let card_style = if is_selected {
                        "cursor: pointer; border: 2px solid #2196F3; border-radius: 6px; padding: 4px; opacity: 1;"
                    } else {
                        "cursor: pointer; border: 2px solid transparent; border-radius: 6px; padding: 4px; opacity: 0.7;"
                    };

                    view! {
                        <div
                            style=card_style
                            on:click=move |_| {
                                if !disabled {
                                    on_change(preset.to_string())
                                }
                            }
                            title=preset
                        >
                            <MatPreview style=style size=40 />
                            <div style="text-align: center; font-size: 0.7rem; margin-top: 2px; color: #666;">
                                {preset}
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
```

#### B3. Update PhotoCard to Use MatPresetPicker

Replace the select dropdown in PhotoCard with the new visual picker:

```rust
// Replace this:
<select ...>...</select>

// With this:
<MatPresetPicker
    current_preset=current_preset.clone()
    on_change=handle_mat_change
    disabled=updating_mat.get()
/>
```

#### B4. Add Live Preview to PhotoCard (Optional Enhancement)

Show the photo thumbnail with the actual mat style applied:

```rust
// In PhotoCard, wrap the image with mat preview
let mat_style = MatStyle::from_preset(&photo.mat_preset);
let container_style = format!(
    "background: {}; padding: 8px;{}",
    mat_style.background_color,
    mat_style.shadow.as_ref().map(|s| format!(" box-shadow: {};", s)).unwrap_or_default()
);

view! {
    <div style="border: 1px solid #e0e0e0; border-radius: 8px; overflow: hidden; background: white;">
        <div style=container_style>
            <img
                src=photo.url.clone()
                style="width: 100%; height: 134px; object-fit: cover; display: block;"
                loading="lazy"
            />
        </div>
        // ... rest of card
    }
}
```

### Implementation Steps

| Step | Description |
|------|-------------|
| B1 | Create MatPreview component |
| B2 | Create MatPresetPicker component |
| B3 | Update PhotoCard to use MatPresetPicker |
| B4 | (Optional) Add live preview to photo thumbnail |
| B5 | Test visual appearance across all presets |

---

## File Changes Summary

### Part A (Tests)

| File | Change |
|------|--------|
| `crates/common/src/lib.rs` | Add #[cfg(test)] module with MatStyle tests |
| `src/test_helpers.rs` | Update seed_photo to include mat_preset column |
| `tests/api.rs` | Add mat preset API tests (7+ new tests) |

### Part B (Preview)

| File | Change |
|------|--------|
| `crates/frontend-admin/src/main.rs` | Add MatPreview, MatPresetPicker components; update PhotoCard |

---

## Testing the Preview Feature

1. Build and run the admin panel
2. Navigate to Photos tab
3. Verify each photo card shows:
   - Visual mat preset picker (not dropdown)
   - All 6 presets with correct colors
   - Selection highlight on current preset
   - Smooth transitions when changing presets
4. Verify the photo thumbnail shows the mat effect (if B4 implemented)
5. Test on mobile viewport to ensure responsive layout

---

## Future Enhancements (Out of Scope)

- Animated transitions between mat styles on the viewer
- Custom user-defined mat presets
- Per-album default mat preset
- Mat style history/undo
