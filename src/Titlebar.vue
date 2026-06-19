<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
const isMaximized = ref(false);

onMounted(async () => {
  isMaximized.value = await appWindow.isMaximized();
  appWindow.onResized(async () => {
    isMaximized.value = await appWindow.isMaximized();
  });
});

const startDrag = () => appWindow.startDragging();
const minimize = () => appWindow.minimize();
const toggleMaximize = async () => {
  if (isMaximized.value) {
    await appWindow.unmaximize();
  } else {
    await appWindow.maximize();
  }
};
const close = () => appWindow.close();
</script>

<template>
  <div data-tauri-drag-region class="titlebar" @mousedown="startDrag">
    <div data-tauri-drag-region class="titlebar-title">
      BlockNet
    </div>
    <div class="titlebar-controls">
      <button class="titlebar-button" @mousedown.stop @click="minimize" title="Свернуть">
        <div class="titlebar-icon minus-icon"></div>
      </button>
      <button class="titlebar-button" @mousedown.stop @click="toggleMaximize" title="Развернуть">
        <div class="titlebar-icon square-icon"></div>
      </button>
      <button class="titlebar-button tb-danger" @mousedown.stop @click="close" title="Закрыть">
        <div class="titlebar-icon close-icon"></div>
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 40px;
  background: var(--bg);
  border-bottom: 1px solid var(--line);
  display: flex;
  justify-content: space-between;
  align-items: center;
  z-index: 9999;
  user-select: none;
}

.titlebar-title {
  padding-left: 20px;
  font-size: 11px;
  font-weight: 700;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.2em;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
}

.titlebar-controls {
  display: flex;
  height: 100%;
  flex-shrink: 0;
}

.titlebar-button {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  width: 46px;
  height: 100%;
  color: var(--text-muted);
  background: transparent;
  border: none;
  padding: 0;
  margin: 0;
  outline: none;
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}

.titlebar-button:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text);
}

.titlebar-button.tb-danger:hover {
  background: #e81123;
  color: white;
}

.titlebar-icon {
  background: currentColor;
}

.minus-icon {
  width: 16px;
  height: 16px;
  mask: url('/src/assets/icons/minus.svg') center/contain no-repeat;
  -webkit-mask: url('/src/assets/icons/minus.svg') center/contain no-repeat;
}

.square-icon {
  width: 12px;
  height: 12px;
  mask: url('/src/assets/icons/square.svg') center/contain no-repeat;
  -webkit-mask: url('/src/assets/icons/square.svg') center/contain no-repeat;
}

.close-icon {
  width: 16px;
  height: 16px;
  mask: url('/src/assets/icons/x.svg') center/contain no-repeat;
  -webkit-mask: url('/src/assets/icons/x.svg') center/contain no-repeat;
}
</style>
