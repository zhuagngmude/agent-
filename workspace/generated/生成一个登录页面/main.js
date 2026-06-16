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

    // 模拟登录验证
    if (username === 'admin' && password === '123456') {
      // 记住我功能
      if (rememberCheckbox.checked) {
        localStorage.setItem('rememberedUsername', username);
      } else {
        localStorage.removeItem('rememberedUsername');
      }

      loginBtn.textContent = '登录中...';
      loginBtn.disabled = true;

      setTimeout(function() {
        alert('登录成功！欢迎回来，' + username);
        loginBtn.textContent = '登录';
        loginBtn.disabled = false;
        // 实际项目中会跳转到首页
        // window.location.href = '/dashboard';
      }, 1000);
    } else {
      alert('用户名或密码错误（提示：admin / 123456）');
    }
  });

  // 输入框自动聚焦
  usernameInput.focus();
});