(function() {
  const todoInput = document.getElementById('todo-input');
  const addBtn = document.getElementById('add-btn');
  const todoList = document.getElementById('todo-list');
  const itemCount = document.getElementById('item-count');
  const clearBtn = document.getElementById('clear-btn');

  let todos = [];

  function loadTodos() {
    const stored = localStorage.getItem('todos');
    if (stored) {
      try {
        todos = JSON.parse(stored);
      } catch(e) {
        todos = [];
      }
    }
  }

  function saveTodos() {
    localStorage.setItem('todos', JSON.stringify(todos));
  }

  function render() {
    todoList.innerHTML = '';
    todos.forEach((todo, index) => {
      const li = document.createElement('li');
      li.className = 'todo-item';

      const checkbox = document.createElement('input');
      checkbox.type = 'checkbox';
      checkbox.className = 'checkbox';
      checkbox.checked = todo.completed;
      checkbox.addEventListener('change', function() {
        todo.completed = this.checked;
        saveTodos();
        render();
      });

      const textSpan = document.createElement('span');
      textSpan.className = 'todo-text' + (todo.completed ? ' completed' : '');
      textSpan.textContent = todo.text;

      const deleteBtn = document.createElement('button');
      deleteBtn.className = 'delete-btn';
      deleteBtn.textContent = '✕';
      deleteBtn.addEventListener('click', function() {
        todos.splice(index, 1);
        saveTodos();
        render();
      });

      li.appendChild(checkbox);
      li.appendChild(textSpan);
      li.appendChild(deleteBtn);
      todoList.appendChild(li);
    });

    const activeCount = todos.filter(t => !t.completed).length;
    itemCount.textContent = activeCount + ' 项待办';
  }

  function addTodo() {
    const text = todoInput.value.trim();
    if (text === '') {
      alert('请输入待办事项');
      return;
    }
    todos.push({ text: text, completed: false });
    saveTodos();
    render();
    todoInput.value = '';
    todoInput.focus();
  }

  function clearCompleted() {
    todos = todos.filter(t => !t.completed);
    saveTodos();
    render();
  }

  addBtn.addEventListener('click', addTodo);
  todoInput.addEventListener('keypress', function(e) {
    if (e.key === 'Enter') {
      addTodo();
    }
  });
  clearBtn.addEventListener('click', clearCompleted);

  loadTodos();
  render();
})();