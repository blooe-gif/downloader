import { invoke } from '@tauri-apps/api/core';

const status = document.getElementById('status');
const rows = document.getElementById('rows');

async function refresh() {
  const tasks = await invoke('list_tasks_cmd');
  rows.innerHTML = tasks.map((t) => `
    <tr>
      <td>${t.id}</td>
      <td>${t.status}</td>
      <td>${t.priority.toFixed(2)}</td>
      <td>${Math.floor(t.file_size / (1024 * 1024))}</td>
      <td>${t.output_path}</td>
    </tr>`).join('');
}

document.getElementById('queue').addEventListener('click', async () => {
  const url = document.getElementById('url').value.trim();
  const output = document.getElementById('out').value.trim();
  if (!url) return;
  const id = await invoke('queue_task_cmd', { url, output: output || null });
  status.textContent = `Queued task #${id}`;
  await refresh();
});

document.getElementById('run').addEventListener('click', async () => {
  status.textContent = 'Running next queued task...';
  const msg = await invoke('run_next_cmd');
  status.textContent = msg;
  await refresh();
});

document.getElementById('refresh').addEventListener('click', refresh);

refresh().catch((e) => {
  status.textContent = `Startup error: ${e}`;
});
