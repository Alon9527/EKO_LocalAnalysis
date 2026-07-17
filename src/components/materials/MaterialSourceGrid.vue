<script setup lang="ts">
import { reactive, watch } from "vue";
import { useRouter } from "vue-router";
import { Picture as PictureIcon, ArrowRight } from "@element-plus/icons-vue";
import { api, type MaterialSourceVariant } from "@/lib/api";
import { useGalleryStore } from "@/stores/gallery";

const props = defineProps<{ sources: MaterialSourceVariant[] }>();
const router = useRouter();
const galleryStore = useGalleryStore();
const thumbnails = reactive<Record<string, string>>({});
const unavailable = reactive<Record<string, boolean>>({});
let requestVersion = 0;

watch(
  () => props.sources,
  async (sources) => {
    const version = ++requestVersion;
    await Promise.all(
      sources.map(async (source) => {
        if (thumbnails[source.id] || unavailable[source.id]) return;
        try {
          const dataUrl = await api.readThumbnailAsDataUrl(source.thumbnailId);
          if (version === requestVersion) thumbnails[source.id] = dataUrl;
        } catch {
          if (version === requestVersion) unavailable[source.id] = true;
        }
      }),
    );
  },
  { immediate: true },
);

async function openHistory(historyId: string) {
  await router.push({ path: "/gallery", query: { history: historyId } });
  await galleryStore.load();
  galleryStore.openDetail(historyId);
}
</script>

<template>
  <div class="source-grid">
    <article v-for="source in sources" :key="source.id" class="source-item">
      <div class="source-item__preview">
        <img
          v-if="thumbnails[source.id]"
          :src="thumbnails[source.id]"
          alt="来源图片缩略图"
        />
        <div v-else class="source-item__missing">
          <el-icon :size="22"><PictureIcon /></el-icon>
          <span>{{ unavailable[source.id] ? "图片不可用" : "正在读取" }}</span>
        </div>
      </div>

      <div class="source-item__body">
        <code :title="source.fieldPath">{{ source.fieldPath }}</code>
        <p>{{ source.promptZh }}</p>
        <p v-if="source.promptEn" class="source-item__english">{{ source.promptEn }}</p>
        <RouterLink
          custom
          :to="{ path: '/gallery', query: { history: source.historyId } }"
          v-slot="{ href }"
        >
          <a
            :href="href"
            :data-testid="`open-history-${source.id}`"
            class="source-item__link"
            @click.prevent="openHistory(source.historyId)"
          >
            查看原分析
            <el-icon :size="13"><ArrowRight /></el-icon>
          </a>
        </RouterLink>
      </div>
    </article>
  </div>
</template>

<style scoped>
.source-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: 10px;
}

.source-item {
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.025);
}

.source-item__preview {
  aspect-ratio: 4 / 3;
  overflow: hidden;
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  background: rgba(0, 0, 0, 0.24);
}

.source-item__preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.source-item__missing {
  display: flex;
  width: 100%;
  height: 100%;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 7px;
  color: rgba(255, 255, 255, 0.34);
  font-size: 12px;
}

.source-item__body {
  padding: 10px;
}

.source-item__body code {
  display: block;
  overflow: hidden;
  color: rgba(94, 234, 212, 0.72);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.source-item__body p {
  display: -webkit-box;
  overflow: hidden;
  margin: 7px 0 0;
  color: rgba(255, 255, 255, 0.72);
  font-size: 12px;
  line-height: 1.55;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.source-item__body .source-item__english {
  color: rgba(255, 255, 255, 0.42);
}

.source-item__link {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  margin-top: 9px;
  color: #5eead4;
  font-size: 12px;
  text-decoration: none;
}

.source-item__link:hover {
  color: #99f6e4;
}
</style>
