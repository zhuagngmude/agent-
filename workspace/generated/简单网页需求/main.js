document.addEventListener('DOMContentLoaded', function() {
  const button = document.getElementById('actionBtn');
  const message = document.getElementById('message');

  button.addEventListener('click', function() {
    const now = new Date();
    const timeStr = now.toLocaleTimeString('zh-CN', { hour12: false });
    message.textContent = '你于 ' + timeStr + ' 点击了按钮！';
  });
});