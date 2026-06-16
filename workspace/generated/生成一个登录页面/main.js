document.addEventListener('DOMContentLoaded', function() {
  const loginForm = document.getElementById('loginForm');
  const usernameInput = document.getElementById('username');
  const passwordInput = document.getElementById('password');
  const rememberCheckbox = document.getElementById('remember');
  const loginBtn = document.getElementById('loginBtn');

  // 检查是否有保存的凭据
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
      // 模拟验证
      if (username === 'admin' && password === '123456') {
        // 记住我功能
        if (rememberCheckbox.checked) {
          localStorage.setItem('rememberedUsername', username);
        } else {
          localStorage.removeItem('rememberedUsername');
        }

        alert('登录成功！欢迎回来，' + username);
        // 在实际应用中，这里会跳转到主页
        // window.location.href = '/dashboard';
      } else {
        alert('用户名或密码错误，请重试');
      }

      loginBtn.textContent = '登录';
      loginBtn.disabled = false;
    }, 1000);
  });

  // 回车键提交
  passwordInput.addEventListener('keydown', function(event) {
    if (event.key === 'Enter') {
      loginForm.dispatchEvent(new Event('submit'));
    }
  });
});