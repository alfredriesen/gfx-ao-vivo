<script setup lang="ts">
import {onMounted, ref} from "vue";
const style = ref({ width: '100px', height: '100px', background: 'red' });

onMounted(() => {
  const ws = new WebSocket('ws://127.0.0.1:9000');
  console.log(ws);
  ws.onmessage = (event) => {
    console.log(event);
    const data = JSON.parse(event.data);
    if (data.action === 'play') {
      style.value.background = 'green';
    }
  };
});
</script>

<template>
  <div>
    <div :style="style" class="box"></div>
    <a href="https://vite.dev" target="_blank">
      <img src="/vite.svg" class="logo" alt="Vite logo" />
    </a>
    <a href="https://vuejs.org/" target="_blank">
      <img src="./assets/vue.svg" class="logo vue" alt="Vue logo" />
    </a>
  </div>
</template>

<style scoped>
.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: filter 300ms;
}
.logo:hover {
  filter: drop-shadow(0 0 2em #646cffaa);
}
.logo.vue:hover {
  filter: drop-shadow(0 0 2em #42b883aa);
}
.box { transition: all 0.3s ease; }
</style>
