<script setup lang="ts">
import { computed } from "vue";
import { Star, StarFilled } from "@element-plus/icons-vue";
import type { MaterialAsset } from "@/lib/api";
import {
  MATERIAL_CATEGORY_LABELS,
  materialDisplayName,
  materialPromptZh,
} from "@/lib/materials";

const props = defineProps<{ asset: MaterialAsset }>();
const emit = defineEmits<{
  open: [id: string];
  "toggle-favorite": [id: string, favorite: boolean];
}>();

const displayName = computed(() => materialDisplayName(props.asset));
const promptZh = computed(() => materialPromptZh(props.asset));

function toggleFavorite() {
  emit("toggle-favorite", props.asset.id, !props.asset.userOverride.favorite);
}
</script>

<template>
  <article class="material-card">
    <header class="material-card__header">
      <span class="material-card__category">
        {{ MATERIAL_CATEGORY_LABELS[asset.category] }}
      </span>
      <button
        data-testid="favorite"
        type="button"
        class="material-card__favorite"
        :class="{ 'is-favorite': asset.userOverride.favorite }"
        :aria-label="asset.userOverride.favorite ? '取消收藏' : '收藏素材'"
        :title="asset.userOverride.favorite ? '取消收藏' : '收藏素材'"
        @click="toggleFavorite"
      >
        <el-icon :size="17">
          <StarFilled v-if="asset.userOverride.favorite" />
          <Star v-else />
        </el-icon>
      </button>
    </header>
    <button
      data-testid="material-card"
      type="button"
      class="material-card__open"
      :aria-label="`打开素材 ${displayName}`"
      @click="emit('open', asset.id)"
    >
      <h2 class="material-card__title" :title="displayName">{{ displayName }}</h2>
      <p class="material-card__prompt" :title="promptZh">{{ promptZh }}</p>

      <footer class="material-card__footer">
        <span>{{ asset.sources.length }} 个来源</span>
        <span v-if="asset.userOverride.manuallyEdited" class="material-card__edited">已编辑</span>
      </footer>
    </button>
  </article>
</template>

<style scoped>
.material-card {
  display: flex;
  min-height: 184px;
  flex-direction: column;
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 8px;
  background: rgba(18, 21, 28, 0.86);
  padding: 15px;
  color: rgba(255, 255, 255, 0.9);
  transition: border-color 160ms ease, background-color 160ms ease, transform 160ms ease;
}

.material-card:hover,
.material-card:focus-within {
  border-color: rgba(45, 212, 191, 0.42);
  background: rgba(22, 27, 34, 0.96);
  transform: translateY(-1px);
}


.material-card__open {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  border: 0;
  outline: 0;
  background: transparent;
  padding: 0;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.material-card__open:focus-visible {
  outline: 2px solid rgba(45, 212, 191, 0.55);
  outline-offset: 5px;
}
.material-card__header,
.material-card__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.material-card__category {
  color: #5eead4;
  font-size: 12px;
  font-weight: 650;
}

.material-card__favorite {
  display: inline-flex;
  width: 30px;
  height: 30px;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: rgba(255, 255, 255, 0.42);
  cursor: pointer;
}

.material-card__favorite:hover {
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.82);
}

.material-card__favorite.is-favorite {
  color: #fbbf24;
}

.material-card__title {
  overflow: hidden;
  margin: 13px 0 7px;
  color: rgba(255, 255, 255, 0.94);
  font-size: 16px;
  font-weight: 650;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.material-card__prompt {
  display: -webkit-box;
  overflow: hidden;
  margin: 0;
  color: rgba(255, 255, 255, 0.58);
  font-size: 13px;
  line-height: 1.65;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.material-card__footer {
  margin-top: auto;
  padding-top: 14px;
  color: rgba(255, 255, 255, 0.38);
  font-size: 12px;
}

.material-card__edited {
  color: rgba(94, 234, 212, 0.64);
}
</style>
