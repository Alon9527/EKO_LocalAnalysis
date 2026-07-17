# Materials Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn structured image-analysis history into a persistent, searchable prompt-material library, and expose the same granular assets in each history detail drawer.

**Architecture:** Rust owns extraction, normalization, source tracking, persistence, and manual overrides in `materials-index.json`. Tauri commands expose a typed asset API. Vue uses a dedicated Pinia store and shared breakdown/detail components so History and Materials Library read the same backend index. Existing `history.json` remains the source of truth; no API request is made during extraction or rebuild.

**Tech Stack:** Rust 2021, Serde/serde_json, Tauri 2 commands, Vue 3 Composition API, Pinia 3, Vue Router 4, Element Plus, Tailwind CSS 4, Vitest, Vue Test Utils, jsdom.

---

## Working Agreements

- Preserve the user's existing uncommitted edits in `src/views/GalleryView.vue` and `src/views/SingleView.vue`; stage only the files named by each task.
- Do not rewrite the analysis JSON schema or call an AI provider while building the index.
- Never invent English fragments. Store one only when the local clause matcher finds a conservative match in the existing `prompt_en`.
- Generated fields and user overrides remain separate. Rebuilds may replace generated fields, but must preserve manual names, prompts, aliases, favorites, merges, and splits.
- Before adding Vitest dependencies, audit the package names, maintainers, licenses, and current versions, then request the user's approval for the networked install.
- Use focused tests first, then implementation, then the production checks at the end.

## Task 1: Establish Frontend Test Infrastructure

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`
- Create: `src/test/setup.ts`
- Create: `src/lib/materials.test.ts`

- [ ] **Step 1: Audit the proposed test-only packages**

Run with network approval:

```powershell
npm view vitest version license maintainers --json
npm view @vue/test-utils version license maintainers --json
npm view jsdom version license maintainers --json
```

Expected: each package resolves from npm, has an OSI-compatible license, and the names match the official Vitest/Vue Test Utils/jsdom packages. Record the versions selected in `package-lock.json`.

- [ ] **Step 2: Install the approved test dependencies**

Run with explicit user approval:

```powershell
npm install --save-dev vitest @vue/test-utils jsdom
```

Expected: `package.json` and `package-lock.json` change only for the three approved development dependencies and their transitive dependencies.

- [ ] **Step 3: Add the test command and jsdom configuration**

Add to `package.json` scripts:

```json
"test": "vitest run",
"test:watch": "vitest"
```

Preserve the existing Tauri server/HMR configuration in `vite.config.ts`. Change only the `defineConfig` import:

```ts
import { defineConfig } from "vitest/config";
```

Then add this property to the object returned by the existing async `defineConfig` callback, immediately before `server`:

```ts
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    restoreMocks: true,
  },
```

Do not remove `host`, `clearScreen`, `server.port`, `strictPort`, HMR, or the `src-tauri` watch exclusion.

In `src/test/setup.ts`, reset DOM and local storage after each test:

```ts
import { afterEach } from "vitest";

afterEach(() => {
  document.body.innerHTML = "";
  localStorage.clear();
});
```

- [ ] **Step 4: Prove the runner works with a failing smoke behavior test**

Create `src/lib/materials.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { groupMaterialsByCategory } from "@/lib/materials";

describe("groupMaterialsByCategory", () => {
  it("groups assets without changing their order", () => {
    const assets = [
      { id: "light-1", category: "lighting" },
      { id: "element-1", category: "element" },
      { id: "light-2", category: "lighting" },
    ] as any[];

    expect(groupMaterialsByCategory(assets)).toEqual({
      lighting: [assets[0], assets[2]],
      element: [assets[1]],
    });
  });
});
```

Run:

```powershell
npm test -- src/lib/materials.test.ts
```

Expected: FAIL because `src/lib/materials.ts` does not exist.

- [ ] **Step 5: Add the smallest grouping helper**

Create `src/lib/materials.ts` with a typed reducer and no UI dependencies. The final type import will come from `src/lib/api.ts` in Task 5:

```ts
export function groupMaterialsByCategory<T extends { category: string }>(assets: T[]) {
  return assets.reduce<Record<string, T[]>>((groups, asset) => {
    (groups[asset.category] ||= []).push(asset);
    return groups;
  }, {});
}
```

Run:

```powershell
npm test -- src/lib/materials.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit only the test harness**

```powershell
git add package.json package-lock.json vite.config.ts src/test/setup.ts src/lib/materials.ts src/lib/materials.test.ts
git commit -m "test: add frontend test harness"
```

## Task 2: Build the Rust Extraction Domain

**Files:**
- Create: `src-tauri/src/materials.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing extraction and normalization tests**

At the bottom of `materials.rs`, add unit tests for:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn structured_history() -> HistoryItem {
        HistoryItem {
            id: "history-1".into(),
            file_name: "chair.jpg".into(),
            file_path: String::new(),
            image_url: String::new(),
            source_type: "file".into(),
            aspect_ratio: Some("4:3".into()),
            contains_people: Some(false),
            reconstructed_prompt: Some(serde_json::json!({
                "global_scene": {
                    "art_style": "商业室内摄影",
                    "atmosphere": "安静、温暖",
                    "color_palette": ["胡桃木棕", "暖白色"],
                    "lighting": "左侧窗户投下柔和暖光"
                },
                "composition": {
                    "camera_angle": "平视",
                    "focal_length": "标准镜头",
                    "framing": "主体居中",
                    "depth_of_field": "中等景深"
                },
                "entities": [{
                    "label": "休闲椅",
                    "appearance": "焦糖色皮革，弧形扶手",
                    "sub_elements": ["圆形坐垫"]
                }],
                "environment_details": {
                    "foreground": "深色边桌",
                    "midground": "休闲椅",
                    "background": "木饰面墙"
                },
                "technical_specs": {
                    "texture_fidelity": "细腻皮革纹理，清晰木纹",
                    "render_engine_style": "真实商业摄影",
                    "vfx": []
                }
            })),
            reconstructed_prompt_zh: None,
            quality_notes: None,
            prompt_en: Some("A caramel leather lounge chair with curved arms, warm window light, eye-level standard-lens interior photography.".into()),
            prompt_zh: Some("焦糖色皮革休闲椅，弧形扶手，暖色窗光，平视标准镜头室内摄影。".into()),
            quality_score: 84,
            quality_label: "较强".into(),
            quality_breakdown: serde_json::json!({}),
            quality_warnings: vec![],
            model: "gpt-5.4".into(),
            provider: "openai-compatible".into(),
            elapsed_ms: 100,
            favorite: false,
            created_at: 1000,
        }
    }

    #[test]
    fn extracts_all_supported_categories_with_source_paths() {
        let assets = extract_assets(&structured_history());
        let categories = assets.iter().map(|a| a.category).collect::<std::collections::HashSet<_>>();
        assert_eq!(categories.len(), 8);
        assert!(assets.iter().any(|a| a.category == MaterialCategory::Element && a.generated_name == "休闲椅"));
        assert!(assets.iter().any(|a| a.category == MaterialCategory::Material));
        assert!(assets.iter().all(|a| a.sources.iter().all(|s| s.history_id == "history-1")));
        assert!(assets.iter().any(|a| a.sources.iter().any(|s| s.field_path == "entities[0].label")));
    }

    #[test]
    fn normalization_merges_exact_values_but_not_fuzzy_values() {
        assert_eq!(normalize_key("  暖白色， "), normalize_key("暖白色"));
        assert_ne!(normalize_key("暖白色"), normalize_key("米白色"));
    }

    #[test]
    fn full_prompt_only_history_produces_no_assets() {
        let mut item = structured_history();
        item.reconstructed_prompt = None;
        assert!(extract_assets(&item).is_empty());
    }

    #[test]
    fn english_fragment_is_kept_only_for_a_safe_clause_match() {
        let assets = extract_assets(&structured_history());
        let chair = assets.iter().find(|a| a.generated_name == "休闲椅").unwrap();
        assert!(chair.generated_prompt_en.as_deref().unwrap_or("").contains("caramel leather lounge chair"));
        let color = assets.iter().find(|a| a.generated_name == "暖白色").unwrap();
        assert_eq!(color.generated_prompt_en, None);
    }
}
```

Run:

```powershell
cargo test materials --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because the module and domain functions do not exist.

- [ ] **Step 2: Add the serializable domain types**

Implement in `materials.rs`:

```rust
use crate::storage::HistoryItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialCategory {
    Element,
    Material,
    Color,
    Lighting,
    Camera,
    Composition,
    Style,
    Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialSourceVariant {
    pub id: String,
    pub history_id: String,
    pub thumbnail_id: String,
    pub field_path: String,
    pub prompt_zh: String,
    pub prompt_en: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialOverride {
    pub display_name: Option<String>,
    pub prompt_zh: Option<String>,
    pub prompt_en: Option<String>,
    pub aliases: Vec<String>,
    pub favorite: bool,
    pub manually_edited: bool,
    pub merged_into: Option<String>,
    pub split_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialAsset {
    pub id: String,
    pub category: MaterialCategory,
    pub generated_name: String,
    pub generated_explanation: String,
    pub generated_prompt_zh: String,
    pub generated_prompt_en: Option<String>,
    pub generated_aliases: Vec<String>,
    pub user_override: MaterialOverride,
    pub sources: Vec<MaterialSourceVariant>,
    pub created_at: u64,
    pub updated_at: u64,
}
```

- [ ] **Step 3: Implement deterministic extraction**

Implement these pure functions in `materials.rs`:

```rust
pub fn extract_assets(item: &HistoryItem) -> Vec<MaterialAsset>;
fn extract_entities(item: &HistoryItem, root: &serde_json::Value, output: &mut Vec<MaterialAsset>);
fn extract_scene(item: &HistoryItem, root: &serde_json::Value, output: &mut Vec<MaterialAsset>);
fn extract_composition(item: &HistoryItem, root: &serde_json::Value, output: &mut Vec<MaterialAsset>);
fn extract_environment(item: &HistoryItem, root: &serde_json::Value, output: &mut Vec<MaterialAsset>);
fn extract_technical(item: &HistoryItem, root: &serde_json::Value, output: &mut Vec<MaterialAsset>);
fn split_list_like(value: &str) -> Vec<String>;
fn safe_english_clause(prompt_en: Option<&str>, category: MaterialCategory, chinese_fragment: &str) -> Option<String>;
pub fn normalize_key(value: &str) -> String;
fn stable_asset_id(category: MaterialCategory, normalized: &str) -> String;
fn stable_source_id(history_id: &str, field_path: &str, normalized: &str) -> String;
```

Use category plus a local FNV-1a hash of the normalized key for stable IDs. Split arrays directly; split strings only on Chinese/English list punctuation when every resulting segment is non-empty. Keep the unsplit original in `MaterialSourceVariant.prompt_zh`.

For safe English matching, split `prompt_en` into comma/semicolon clauses and return a clause only when it contains an unambiguous local category cue and at least one mapped noun from the small built-in alias table. Return `None` when confidence is insufficient.

- [ ] **Step 4: Register the module and run the tests**

Add `mod materials;` to `src-tauri/src/lib.rs`.

Run:

```powershell
cargo test materials --manifest-path src-tauri/Cargo.toml
```

Expected: all four extraction tests PASS.

- [ ] **Step 5: Commit the extraction domain**

```powershell
git add src-tauri/src/materials.rs src-tauri/src/lib.rs
git commit -m "feat: extract prompt materials from history"
```

## Task 3: Persist the Index and Preserve User State

**Files:**
- Modify: `src-tauri/src/materials.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Write failing persistence and rebuild tests**

Add tests using a unique directory under `std::env::temp_dir()` and pass the path explicitly into storage helpers. Cover:

```rust
#[test]
fn rebuild_preserves_manual_overrides_and_favorites();

#[test]
fn exact_assets_merge_sources_across_history_items();

#[test]
fn deleting_one_history_source_keeps_shared_asset();

#[test]
fn orphaned_manual_asset_survives_source_deletion();

#[test]
fn split_moves_selected_variants_to_a_user_asset();

#[test]
fn invalid_primary_index_falls_back_to_last_valid_backup();
```

Run:

```powershell
cargo test materials --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because index persistence and mutation APIs do not exist.

- [ ] **Step 2: Add index paths and versioned file types**

In `storage.rs`, expose only project-data paths:

```rust
pub fn materials_index_path() -> PathBuf { data_dir().join("materials-index.json") }
pub fn materials_index_backup_path() -> PathBuf { data_dir().join("materials-index.backup.json") }
```

In `materials.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialIndex {
    pub schema_version: u32,
    pub history_fingerprint: String,
    pub assets: Vec<MaterialAsset>,
    pub warnings: Vec<MaterialIndexWarning>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialIndexWarning {
    pub history_id: String,
    pub message: String,
}
```

Set `schema_version` to `1`. Compute `history_fingerprint` from each history id, `created_at`, and serialized `reconstructed_prompt` using the same deterministic FNV-1a helper.

- [ ] **Step 3: Implement recoverable persistence**

Implement path-injected helpers for tests and production wrappers for real app paths:

```rust
fn load_index_from(primary: &Path, backup: &Path) -> Result<MaterialIndex, AnyError>;
fn save_index_to(index: &MaterialIndex, primary: &Path, backup: &Path) -> Result<(), AnyError>;
pub fn load_index() -> Result<MaterialIndex, AnyError>;
pub fn ensure_index(history: &[HistoryItem]) -> Result<MaterialIndex, AnyError>;
pub fn rebuild_index(history: &[HistoryItem]) -> Result<MaterialIndex, AnyError>;
```

Write and deserialize a temporary JSON file before replacing the primary. Preserve the previous valid primary as the backup. If the primary cannot be parsed, load the backup and surface a warning. A failed new rebuild must return an error without discarding the last valid primary/backup pair.

- [ ] **Step 4: Implement merge, split, update, and source removal**

Add pure mutations plus persistence wrappers:

```rust
pub fn apply_patch(index: &mut MaterialIndex, id: &str, patch: MaterialPatch) -> Result<MaterialAsset, AnyError>;
pub fn merge_assets(index: &mut MaterialIndex, ids: &[String], display_name: Option<String>) -> Result<MaterialAsset, AnyError>;
pub fn split_asset(index: &mut MaterialIndex, id: &str, source_ids: &[String], display_name: String) -> Result<Vec<MaterialAsset>, AnyError>;
pub fn upsert_history_source(index: &mut MaterialIndex, item: &HistoryItem);
pub fn remove_history_sources(index: &mut MaterialIndex, ids: &[String]);
pub fn clear_generated_sources(index: &mut MaterialIndex);
```

Validation rules:

- reject blank manual names and blank manual prompt fragments;
- reject merge sets with fewer than two ids or mixed categories;
- reject split sets that are empty, unknown, or include every source;
- retain a zero-source asset only when favorited, edited, merged, split, or otherwise user-managed;
- preserve source order by newest `created_at`, then stable source id.

- [ ] **Step 5: Run focused and full Rust tests**

```powershell
cargo test materials --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all material tests and existing importer/bridge tests PASS.

- [ ] **Step 6: Commit persistence**

```powershell
git add src-tauri/src/materials.rs src-tauri/src/storage.rs
git commit -m "feat: persist reusable prompt materials"
```

## Task 4: Expose Tauri Commands and Keep the Index in Sync

**Files:**
- Modify: `src-tauri/src/materials.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/analyzer.rs`
- Modify: `src-tauri/src/importer.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Write failing command-serialization and sync tests**

Add Rust tests that assert:

- list response serializes categories as `snake_case` and fields as `camelCase`;
- query filters category, text, favorites, and minimum source count;
- `upsert_history_source` adds a newly analyzed record without losing overrides;
- importing several records adds each source once;
- deleting history ids removes only those variants;
- clearing history retains only user-managed orphan assets.

Run:

```powershell
cargo test materials --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL until query and sync functions exist.

- [ ] **Step 2: Add command request/response types**

In `materials.rs`, add:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialQuery {
    pub keyword: Option<String>,
    pub category: Option<MaterialCategory>,
    pub favorite: Option<bool>,
    pub min_sources: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialListResponse {
    pub items: Vec<MaterialAsset>,
    pub total: usize,
    pub stale: bool,
    pub warnings: Vec<MaterialIndexWarning>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialPatch {
    pub display_name: Option<String>,
    pub prompt_zh: Option<String>,
    pub prompt_en: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub favorite: Option<bool>,
}
```

- [ ] **Step 3: Add Tauri commands**

Add handlers in `lib.rs`:

```rust
#[tauri::command]
async fn list_materials(query: materials::MaterialQuery) -> Result<materials::MaterialListResponse, String>;

#[tauri::command]
async fn get_history_materials(history_id: String) -> Result<Vec<materials::MaterialAsset>, String>;

#[tauri::command]
async fn rebuild_material_index() -> Result<materials::MaterialListResponse, String>;

#[tauri::command]
async fn update_material(id: String, patch: materials::MaterialPatch) -> Result<materials::MaterialAsset, String>;

#[tauri::command]
async fn merge_materials(ids: Vec<String>, display_name: Option<String>) -> Result<materials::MaterialAsset, String>;

#[tauri::command]
async fn split_material(id: String, source_ids: Vec<String>, display_name: String) -> Result<Vec<materials::MaterialAsset>, String>;
```

Register all six in `tauri::generate_handler!`.

- [ ] **Step 4: Synchronize every history mutation**

After `storage::add_history_item(item.clone())?` in `analyzer.rs`, call the persisted upsert wrapper.

After imported history and thumbnails are saved in `importer.rs`, upsert only `outcome.items.iter().take(outcome.summary.imported)`.

In `delete_history`, remove index sources only after history deletion succeeds. In `clear_history`, clear generated sources only after history clear succeeds. If index sync fails, return an error message that identifies index sync while leaving the already-saved history readable; the next `ensure_index` must repair it from history.

- [ ] **Step 5: Run Rust verification**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: formatting and all tests PASS.

- [ ] **Step 6: Commit command and sync integration**

```powershell
git add src-tauri/src/materials.rs src-tauri/src/lib.rs src-tauri/src/analyzer.rs src-tauri/src/importer.rs src-tauri/src/storage.rs
git commit -m "feat: sync materials index with history"
```

## Task 5: Add Typed Frontend API and Pinia Store

**Files:**
- Modify: `src/lib/api.ts`
- Modify: `src/lib/materials.ts`
- Modify: `src/lib/materials.test.ts`
- Create: `src/stores/materials.ts`
- Create: `src/stores/materials.test.ts`

- [ ] **Step 1: Write failing query and store tests**

Extend `src/lib/materials.test.ts` to verify category labels and resolved display values. Create `src/stores/materials.test.ts` with a mocked `api` that verifies:

- `load()` forwards keyword/category/favorite/min-source filters;
- `loadForHistory(id)` uses `getHistoryMaterials`;
- `save`, `merge`, and `split` refresh the selected asset and list;
- failed rebuild keeps previous items and exposes an error.

Run:

```powershell
npm test -- src/lib/materials.test.ts src/stores/materials.test.ts
```

Expected: FAIL because material API types and the store are missing.

- [ ] **Step 2: Add API types and wrappers**

In `src/lib/api.ts`, add exact unions and interfaces matching Rust serialization:

```ts
export type MaterialCategory =
  | "element"
  | "material"
  | "color"
  | "lighting"
  | "camera"
  | "composition"
  | "style"
  | "environment";

export interface MaterialSourceVariant {
  id: string;
  historyId: string;
  thumbnailId: string;
  fieldPath: string;
  promptZh: string;
  promptEn?: string;
  createdAt: number;
}

export interface MaterialOverride {
  displayName?: string;
  promptZh?: string;
  promptEn?: string;
  aliases: string[];
  favorite: boolean;
  manuallyEdited: boolean;
  mergedInto?: string;
  splitFrom?: string;
}

export interface MaterialAsset {
  id: string;
  category: MaterialCategory;
  generatedName: string;
  generatedExplanation: string;
  generatedPromptZh: string;
  generatedPromptEn?: string;
  generatedAliases: string[];
  userOverride: MaterialOverride;
  sources: MaterialSourceVariant[];
  createdAt: number;
  updatedAt: number;
}
```

Add `MaterialQuery`, `MaterialPatch`, `MaterialListResponse`, and wrappers for the six Tauri commands. Browser fallback returns an empty list with `stale: false`; mutation methods remain desktop-only.

- [ ] **Step 3: Add resolved-value helpers**

In `src/lib/materials.ts`, add:

```ts
export const MATERIAL_CATEGORY_LABELS: Record<MaterialCategory, string> = {
  element: "元素",
  material: "材质",
  color: "色彩",
  lighting: "光影",
  camera: "镜头",
  composition: "构图",
  style: "风格",
  environment: "环境",
};

export function materialDisplayName(asset: MaterialAsset) {
  return asset.userOverride.displayName || asset.generatedName;
}

export function materialPromptZh(asset: MaterialAsset) {
  return asset.userOverride.promptZh || asset.generatedPromptZh;
}

export function materialPromptEn(asset: MaterialAsset) {
  return asset.userOverride.promptEn || asset.generatedPromptEn || "";
}
```

- [ ] **Step 4: Implement the materials store**

The store state must include:

```ts
const items = ref<MaterialAsset[]>([]);
const historyItems = ref<Record<string, MaterialAsset[]>>({});
const selectedAsset = ref<MaterialAsset | null>(null);
const keyword = ref("");
const category = ref<MaterialCategory | "all">("all");
const favoriteOnly = ref(false);
const minSources = ref<number | undefined>();
const loading = ref(false);
const rebuilding = ref(false);
const error = ref("");
const warnings = ref<MaterialIndexWarning[]>([]);
```

Implement `load`, `loadForHistory`, `openAsset`, `closeAsset`, `saveAsset`, `mergeAssets`, `splitAsset`, and `rebuild`. Debounce search in the view, not the store.

- [ ] **Step 5: Run frontend tests and type check**

```powershell
npm test -- src/lib/materials.test.ts src/stores/materials.test.ts
npm run build
```

Expected: tests and production type/build checks PASS.

- [ ] **Step 6: Commit API and store**

```powershell
git add src/lib/api.ts src/lib/materials.ts src/lib/materials.test.ts src/stores/materials.ts src/stores/materials.test.ts
git commit -m "feat: add materials frontend data layer"
```

## Task 6: Build the Materials Library Page

**Files:**
- Modify: `src/router/index.ts`
- Modify: `src/components/SideBar.vue`
- Create: `src/views/MaterialsView.vue`
- Create: `src/components/materials/MaterialAssetCard.vue`
- Create: `src/components/materials/MaterialDetailDrawer.vue`
- Create: `src/components/materials/MaterialSourceGrid.vue`
- Create: `src/components/materials/MaterialAssetCard.test.ts`
- Create: `src/components/materials/MaterialDetailDrawer.test.ts`
- Create: `src/views/MaterialsView.test.ts`

- [ ] **Step 1: Write failing card and page tests**

Use Vue Test Utils with Element Plus installed on the test app. Verify:

- a card shows category, resolved name, prompt fragment, source count, and favorite state;
- search is debounced and calls `store.load`;
- category/favorite/min-source controls update filters;
- selecting a card opens the drawer;
- the detail drawer saves resolved fields and constrains merge/split selections to valid same-category sources;
- empty structured data explains that new analyses appear automatically;
- rebuild failure leaves current cards visible and shows the error.

Run:

```powershell
npm test -- src/components/materials/MaterialAssetCard.test.ts src/components/materials/MaterialDetailDrawer.test.ts src/views/MaterialsView.test.ts
```

Expected: FAIL because the components and route do not exist.

- [ ] **Step 2: Add navigation and route**

Add to `src/router/index.ts`:

```ts
{ path: "/materials", name: "materials", component: () => import("@/views/MaterialsView.vue") },
```

Add a `Collection` icon item immediately after History in `SideBar.vue`:

```ts
{ path: "/materials", label: "素材库", icon: Collection },
```

- [ ] **Step 3: Build the unframed library workspace**

`MaterialsView.vue` should use:

- a compact top toolbar with one search field;
- category tabs for All plus the eight categories;
- a favorites toggle and source-count select;
- a small overflow menu containing Rebuild Index;
- a responsive grid `repeat(auto-fill,minmax(240px,1fr))`;
- no nested cards and no page-section cards;
- an 8px or smaller radius on asset cards;
- source count and category rendered as compact metadata, not oversized badges.

On mount, call `store.load()`. Watch `route.query.asset`; when present, open that asset after loading. On card click, update the route query to `{ asset: id }` and open the detail drawer.

- [ ] **Step 4: Build card and source-grid components**

`MaterialAssetCard.vue` accepts `asset` and emits `open` and `toggle-favorite`. It truncates the prompt to three lines but keeps a tooltip with the full fragment.

`MaterialSourceGrid.vue` accepts source variants, asynchronously calls `api.readThumbnailAsDataUrl(source.thumbnailId)`, and renders a fixed 4:3 tile. Failed thumbnail loads show an icon and “图片不可用”, while the source prompt and history link remain available.

- [ ] **Step 5: Build the detail drawer**

`MaterialDetailDrawer.vue` provides:

- resolved name and category;
- editable Chinese prompt and optional English prompt tabs;
- aliases input;
- copy actions;
- favorite toggle;
- source variants with field paths and prompt fragments;
- links to `/gallery?history=<historyId>`;
- Rename/Edit save, Merge, and Split actions;
- disabled English editing state when no generated or manual English exists, with a link to the source image's complete English prompt.

Keep merge and split selection inside compact dialogs. Merge lists only assets from the same category. Split requires a non-empty proper subset of source variants.

- [ ] **Step 6: Run component tests and build**

```powershell
npm test -- src/components/materials/MaterialAssetCard.test.ts src/components/materials/MaterialDetailDrawer.test.ts src/views/MaterialsView.test.ts
npm run build
```

Expected: tests and production build PASS.

- [ ] **Step 7: Commit the library UI**

```powershell
git add src/router/index.ts src/components/SideBar.vue src/views/MaterialsView.vue src/components/materials/MaterialAssetCard.vue src/components/materials/MaterialDetailDrawer.vue src/components/materials/MaterialSourceGrid.vue src/components/materials/MaterialAssetCard.test.ts src/components/materials/MaterialDetailDrawer.test.ts src/views/MaterialsView.test.ts
git commit -m "feat: add searchable materials library"
```

## Task 7: Add Analysis Breakdown to History Detail

**Files:**
- Modify: `src/views/GalleryView.vue`
- Modify: `src/stores/gallery.ts`
- Create: `src/components/materials/AnalysisBreakdown.vue`
- Create: `src/components/materials/AnalysisBreakdown.test.ts`

- [ ] **Step 1: Write failing breakdown tests**

Verify:

- assets are grouped in the fixed category order: element, material, color, lighting, camera, composition, style, environment;
- each row shows available-language state, copy, and “在素材库查看”;
- full-prompt-only history shows “暂无结构化分析内容” and never fabricates assets;
- clicking a library link routes to `/materials?asset=<id>`;
- changing the history detail item reloads per-image assets.

Run:

```powershell
npm test -- src/components/materials/AnalysisBreakdown.test.ts
```

Expected: FAIL because the component does not exist.

- [ ] **Step 2: Add top-level detail tabs without removing bilingual prompt tabs**

In `GalleryView.vue`, introduce:

```ts
const detailTab = ref<"prompt" | "breakdown">("prompt");
```

Reset it when `store.detailItem?.id` changes. Wrap the existing Chinese/English prompt tabs under a top-level “完整 Prompt” pane. Add “分析拆解” as the second top-level pane and render `AnalysisBreakdown` with the current history id.

Do not replace or regress the existing selectable textareas, image preview, close button spacing, copy actions, export, or delete behavior.

- [ ] **Step 3: Implement `AnalysisBreakdown.vue`**

On history-id change, call `materialsStore.loadForHistory(id)`. Render compact unframed category sections. Each asset row contains:

- resolved title;
- source fragment;
- `中` or `中 / EN` language indicator;
- copy icon button with tooltip;
- route link to the global asset.

Use a stable minimum row height so copy feedback does not shift the drawer layout.

- [ ] **Step 4: Support direct navigation to a history detail**

In `GalleryView.vue`, watch `route.query.history` after `store.load()` and call `store.openDetail(historyId)`. In `gallery.ts`, make `openDetail` return the found item and leave the drawer closed when the id is missing.

- [ ] **Step 5: Run focused tests and build**

```powershell
npm test -- src/components/materials/AnalysisBreakdown.test.ts
npm run build
```

Expected: tests and build PASS with the existing Gallery/Single changes intact.

- [ ] **Step 6: Commit history integration only**

```powershell
git add src/views/GalleryView.vue src/stores/gallery.ts src/components/materials/AnalysisBreakdown.vue src/components/materials/AnalysisBreakdown.test.ts
git commit -m "feat: show reusable assets in history details"
```

## Task 8: Exercise Editing, Merge, Split, and Rebuild End to End

**Files:**
- Modify: `src/components/materials/MaterialDetailDrawer.test.ts`
- Modify: `src/views/MaterialsView.test.ts`
- Modify: `src-tauri/src/materials.rs`

- [ ] **Step 1: Add failure-path tests**

Cover:

- saving a blank name is rejected and the previous value remains;
- mixed-category merge is rejected;
- split with all sources is rejected;
- rebuild preserves a renamed/favorited asset;
- a missing thumbnail does not hide the prompt source;
- source deletion updates counts and keeps other sources;
- a stale index is rebuilt on first list without an API request.

Run:

```powershell
npm test -- src/components/materials/MaterialDetailDrawer.test.ts src/views/MaterialsView.test.ts
cargo test materials --manifest-path src-tauri/Cargo.toml
```

Expected: at least one test FAIL until error handling and stale-index behavior are complete.

- [ ] **Step 2: Complete validation and user feedback**

Use `ElMessage` for successful copy/save/merge/split/rebuild actions and concise errors. Keep dialogs open on validation failure. Do not clear text fields after a rejected save.

When `list_materials` detects a fingerprint mismatch, call `ensure_index`, preserve old overrides, and return the rebuilt response. It must not call `analyze_image` or any network client.

- [ ] **Step 3: Run all automated checks**

```powershell
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all checks PASS.

- [ ] **Step 4: Commit edge-case completion**

```powershell
git add src/components/materials/MaterialDetailDrawer.test.ts src/views/MaterialsView.test.ts src-tauri/src/materials.rs
git commit -m "test: cover materials library workflows"
```

## Task 9: Visual QA in Browser and Desktop Runtime

**Files:**
- Modify only if QA reveals a defect: files from Tasks 5-8

- [ ] **Step 1: Start the frontend preview**

```powershell
npm run dev -- --host 127.0.0.1
```

Expected: Vite prints an available localhost URL.

- [ ] **Step 2: Verify responsive browser states with Playwright**

At desktop widths 1440x900 and 1920x1080, verify:

- navigation text fits and “素材库” is visible;
- library toolbar does not overlap or wrap incoherently;
- cards remain dense and readable;
- drawer controls remain distinct and clickable;
- History “完整 Prompt / 分析拆解” tabs fit within 520px;
- missing-image and empty-data states are legible;
- no horizontal page scrollbar appears.

At 1024x768, verify the library uses fewer grid columns and the drawer remains usable without covering its close button.

Capture screenshots for each viewport and inspect them. Fix any overlap, clipping, unreadable contrast, or layout shift, then rerun the checks from Task 8.

- [ ] **Step 3: Run the Tauri desktop app**

```powershell
npm run tauri dev
```

Use a copy of local history data and verify:

1. first Materials Library open indexes old structured records without API traffic;
2. a new single-image result appears automatically;
3. an imported ZIP/JSON result appears automatically;
4. rename, edit, favorite, merge, and split survive a manual rebuild;
5. deleting one history record updates source counts;
6. clearing history retains only user-managed assets;
7. History and Materials Library show identical fragments for the same source.

- [ ] **Step 4: Final verification**

```powershell
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
git status --short
git diff --check
```

Expected: all checks PASS; `git diff --check` prints nothing; `git status --short` contains only intentional files and any pre-existing user modifications that were not included in feature commits.

- [ ] **Step 5: Commit QA fixes if needed**

Run `git diff --name-only`, inspect every path, then stage each QA-changed file with a separate `git add` command. Do not use a wildcard or include pre-existing user changes.

```powershell
git commit -m "fix: polish materials library interactions"
```

If QA requires no source change, skip this commit.

## Completion Checklist

- [ ] Existing structured history appears without any AI/API request.
- [ ] New analyses and imports update the index automatically.
- [ ] Delete and clear operations update source links safely.
- [ ] All eight categories are searchable and filterable.
- [ ] History detail and global library use the same indexed assets.
- [ ] Chinese fragments are always available when structured data exists.
- [ ] English fragments appear only after conservative local matching or manual entry.
- [ ] Rename, edit, favorite, merge, and split survive rebuild.
- [ ] Missing thumbnails and incomplete old records have clear states.
- [ ] Frontend tests, production build, Rust formatting, and Rust tests pass.
