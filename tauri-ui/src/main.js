const log = document.getElementById('log');
document.getElementById('btn').addEventListener('click', async () => {
  const url = document.getElementById('url').value.trim();
  if (!url) return;
  log.textContent = `Queue request submitted for: ${url}\n(Connect this button to tauri command invoke)`;
});
