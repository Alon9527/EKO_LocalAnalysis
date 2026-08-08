const statusEl = document.getElementById('status');
const checkButton = document.getElementById('check');

checkButton.addEventListener('click', checkHealth);
checkHealth();

function checkHealth() {
  statusEl.textContent = '\u68c0\u67e5\u4e2d...';
  statusEl.className = '';
  chrome.runtime.sendMessage({ type: 'EKO_HEALTH' }, (response) => {
    if (chrome.runtime.lastError || !response?.ok) {
      statusEl.textContent = '\u672a\u8fde\u63a5';
      statusEl.className = 'is-error';
      return;
    }
    statusEl.textContent = '\u5df2\u8fde\u63a5 v' + (response.version || '-');
    statusEl.className = 'is-ok';
  });
}
