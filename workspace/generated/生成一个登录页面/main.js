document.addEventListener('DOMContentLoaded', function() {
  const loginForm = document.getElementById('loginForm');
  const usernameInput = document.getElementById('username');
  const passwordInput = document.getElementById('password');
  const rememberCheckbox = document.getElementById('remember');
  const loginBtn = document.getElementById('loginBtn');

  // 检查是否有记住的用户名
  const savedUsername = localStorage.getItem('rememberedUsername');
  if (savedUsername) {
    usernameInput.value = savedUsername;
    rememberCheckbox.checked = true;
  }

  loginForm.addEventListener('submit', function(event) {
    event.preventDefault();

    const username = usernameInput.value.trim();
    const password = passwordInput.value.trim();

    if (!username || !password) {
      alert('请输入用户名和密码');
      return;
    }

    // 模拟登录请求
    loginBtn.textContent = '登录中...';
    loginBtn.disabled = true;

    setTimeout(function() {
      // 模拟成功
      if (rememberCheckbox.checked) {
        localStorage.setItem('rememberedUsername', username);
      } else {
        localStorage.removeItem('rememberedUsername');
      }

      alert('登录成功！欢迎回来，' + username + '。');
      loginBtn.textContent = '登录';
      loginBtn.disabled = false;
      // 实际项目中这里会跳转页面
    }, 1500);
  });

  // 输入框获得焦点时清除错误状态
  usernameInput.addEventListener('focus', function() {
    this.style.borderColor = '';
  });
  passwordInput.addEventListener('focus', function() {
    this.style.borderColor = '';
  });
});