<script setup>
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps({
  peers: { type: Array, default: () => [] },
  status: Object
})
const emit = defineEmits(['kick'])

const activePeer = ref(null)

const openPeerModal = (peer) => {
  activePeer.value = peer
}

const closePeerModal = () => {
  activePeer.value = null
}

const kickPeer = async (ip) => {
  try {
    await invoke('kick_player', { ip })
    closePeerModal()
  } catch(e) {
    console.error(e)
  }
}

const transportLabel = (loc) => {
  if (!loc) return 'Неизвестно'
  return loc
}

const pingColor = (ms) => {
  if (ms == null) return 'var(--text-muted)'
  if (ms < 80) return '#4ade80'
  if (ms < 150) return '#facc15'
  return '#f87171'
}
</script>

<template>
  <div>
    <div class="row-head players-head">
      <span>Игрок</span>
      <span>Локация</span>
      <span>Пинг</span>
      <span>Действие</span>
    </div>
    <div id="peer-list" class="peer-list-container">
      <div v-if="!peers || peers.length === 0" style="padding: 16px; color: var(--text-muted); text-align: center; font-size: 14px;">
        Нет подключений
      </div>
      <div v-else v-for="peer in peers" :key="peer.ip" class="peer-row" @click="openPeerModal(peer)">
        <div class="peer-avatar">{{ (peer.ip || '?').slice(0, 2).toUpperCase() }}</div>
        <div class="peer-info">
          <span class="peer-name">{{ peer.ip || 'Unknown' }}</span>
        </div>
        <span class="peer-transport" style="font-size: 12px; color: var(--text-soft);">{{ peer.location || 'Неизвестно' }}</span>
        <span class="peer-ping" :style="{ color: pingColor(peer.rtt_ms), fontVariantNumeric: 'tabular-nums' }">
          {{ peer.rtt_ms != null ? `${peer.rtt_ms} ms` : '— ms' }}
        </span>
        <button class="ghost-button danger-button" style="font-size: 12px; padding: 4px 10px;" @click.stop="kickPeer(peer.ip)">
          Кикнуть
        </button>
      </div>
    </div>

    <!-- Player detail modal -->
    <div v-if="activePeer" class="modal-shell">
      <div class="modal-backdrop" @click="closePeerModal"></div>
      <div class="modal-card" role="dialog" aria-modal="true">
        <div class="panel-head">
          <div class="player-modal-header">
            <div class="player-modal-avatar">{{ (activePeer.ip || '?').slice(0, 2).toUpperCase() }}</div>
            <div>
              <span class="eyebrow">ИНФОРМАЦИЯ ОБ ИГРОКЕ</span>
              <h2>{{ activePeer.ip || 'Unknown' }}</h2>
            </div>
          </div>
          <button @click="closePeerModal" class="icon-button" type="button" aria-label="Закрыть">×</button>
        </div>

        <div class="player-modal-content">
          <div class="player-info-grid">
            <div class="info-block">
              <span>Локация</span>
              <strong>{{ activePeer.location || 'Неизвестно' }}</strong>
            </div>
            <div class="info-block">
              <span>Пинг</span>
              <strong :style="{ color: pingColor(activePeer.rtt_ms) }">
                {{ activePeer.rtt_ms != null ? `${activePeer.rtt_ms} ms` : '—' }}
              </strong>
            </div>
          </div>
        </div>

        <div class="modal-actions">
          <button @click="kickPeer(activePeer.ip)" class="ghost-button danger-button">Кикнуть</button>
          <button @click="closePeerModal" class="primary-button">Закрыть</button>
        </div>
      </div>
    </div>
  </div>
</template>
