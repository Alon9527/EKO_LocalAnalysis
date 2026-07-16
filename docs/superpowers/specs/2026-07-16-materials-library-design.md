# Materials Library Design

## Goal

Turn each saved image analysis into reusable, searchable prompt assets without making another API request. Users can understand one image in detail from History and search reusable prompt fragments across all images from a dedicated Materials Library.

## Confirmed Decisions

- Provide both entry points: a per-image breakdown inside History and a dedicated Materials Library in the left navigation.
- Build a persistent local index from existing structured analysis data.
- Rebuild existing records without API calls; future analyses and imported results update the index incrementally.
- Merge similar assets by default while allowing users to rename, merge, split, edit, and favorite them.
- Keep history records as the source of truth. User edits are stored as overrides and survive index rebuilds.

## Scope

Version one includes element, material, color, lighting, camera, composition, style, and environment assets. It supports search, category filters, source images, available-language prompt fragments, copying, favorites, manual editing, merging, splitting, and rebuilding the index.

Version one does not run a second AI extraction pass, generate embeddings, use cloud synchronization, or rewrite the original analysis JSON.

## Data Model

The application stores the derived index in a separate local JSON file next to history storage.

`MaterialAsset` contains:

- stable asset id and category;
- Chinese display name and optional English name;
- Chinese explanation, the original structured prompt fragment, and an optional matched English fragment;
- normalized aliases used for matching and search;
- favorite state and user-edited fields;
- one or more source variants linked by history id;
- creation and update timestamps.

`MaterialSourceVariant` contains the history id, source thumbnail reference, extracted field path, original structured fragment, optional matched English fragment, and source timestamp.

Manual overrides are stored separately from generated values. Rebuilding the index updates generated entries and source links but preserves renames, favorites, manual prompt edits, merges, and splits.

## Extraction Mapping

- `entities[].label`, `appearance`, and `sub_elements` produce element assets.
- Entity appearance plus `technical_specs.texture_fidelity` produce material and surface assets.
- `global_scene.color_palette` produces color assets.
- `global_scene.lighting` produces lighting assets.
- `composition.camera_angle` and `focal_length` produce camera assets.
- `composition.framing` and `depth_of_field` produce composition assets.
- `global_scene.art_style`, `atmosphere`, and `technical_specs.render_engine_style` produce style assets.
- `environment_details.foreground`, `midground`, and `background` produce environment assets.

Local extraction splits list-like text conservatively on structured arrays and punctuation. The original source fragment is always retained so users can inspect and correct an imperfect split.

Existing structured fields are primarily Chinese. Version one never fabricates an English translation. It stores an English fragment only when a category-specific local clause matcher can safely associate part of the existing `prompt_en` with the asset. Otherwise the asset keeps its Chinese structured fragment and links back to the complete English prompt for reference.

## Matching And Merge Rules

Generated assets are normalized by category, case, whitespace, punctuation, and a small local alias table. Exact normalized matches merge automatically. Fuzzy matching is only suggested to the user and never silently merges two assets.

A merged asset keeps every source variant. Splitting creates independent user-managed assets linked to their selected source variants. Deleting a history record removes only its source link. An asset with no remaining sources is removed unless it is favorited or manually edited, in which case it remains as a user asset.

## User Interface

### History Detail

The detail drawer has two top-level tabs: Complete Prompt and Analysis Breakdown. Analysis Breakdown shows compact sections for elements, materials, colors, lighting, camera, composition, style, and environment. Each item exposes its available prompt fragment, language indicator, copy action, and a link to related entries in the Materials Library.

### Materials Library

The left navigation gains Materials Library. Its main view contains a single search field, category tabs, favorite and source-count filters, and a dense responsive grid of asset cards. Cards show category, title, short prompt fragment, source count, and favorite state.

Selecting a card opens a side detail panel with editable available-language prompt fragments, aliases, all source images, per-source variants, a link to the complete bilingual prompt, copy actions, rename, merge, split, and favorite controls. Search covers titles, aliases, explanations, structured fragments, and matched English fragments.

## Data Flow

On first library open, the backend compares the index revision with history and rebuilds when needed. A completed analysis, imported result, edited history result, or deleted history record marks the index stale and triggers an incremental update. A manual Rebuild Index action is available in the library menu.

The frontend uses a dedicated materials store and backend commands for listing, rebuilding, updating, merging, and splitting assets. History views request per-image assets through the same index so both entry points show identical data.

## Failure And Empty States

- If no structured history data exists, the library explains that new analyses will appear automatically.
- If a record has only a complete prompt, its history detail shows that detailed fields are unavailable; it is not guessed into assets.
- A failed rebuild keeps the last valid index and reports which records could not be processed.
- Missing source images do not remove prompt assets; the source tile shows an unavailable-image state.
- Invalid manual edits are rejected without discarding the previous saved value.

## Testing

- Unit tests cover category extraction, normalization, exact merging, override preservation, split behavior, history deletion, and old records with incomplete data.
- Backend tests cover index persistence, migration, rebuild failure recovery, import synchronization, and command serialization.
- Frontend tests cover search, filters, copy actions, editing, merge/split flows, and per-image navigation.
- Production build and Rust tests must pass before packaging.

## Acceptance Criteria

- Existing structured history appears in the library without an API call.
- New and imported analyses become searchable automatically.
- A user can find and copy the prompt fragment for a material, element, angle, lighting setup, composition, style, color, or environment.
- Per-image breakdown and global library show the same underlying asset data.
- Manual renames, edits, favorites, merges, and splits survive rebuilding.
- Deleting history updates source links without damaging assets still used by other records.
