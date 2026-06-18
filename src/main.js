import { createApp } from 'vue'
import App from './App.vue'
import './styles.css'

window.addEventListener('error', (event) => {
  alert('Global Error: ' + event.message + '\n' + event.filename + ':' + event.lineno);
});
window.addEventListener('unhandledrejection', (event) => {
  alert('Unhandled Rejection: ' + event.reason);
});

const app = createApp(App)
app.mount('#app')
