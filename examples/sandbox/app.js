'use strict';

(function() {
  // Constants
  const STORAGE_KEY = 'taskList';
  
  // DOM references
  const form = document.getElementById('task-form');
  const input = document.getElementById('task-input');
  const taskList = document.getElementById('task-list');

  /**
   * Save all tasks to localStorage
   */
  function saveTasks() {
    const tasks = [];
    taskList.querySelectorAll('li').forEach(function(li) {
      tasks.push({
        text: li.querySelector('.task-text').textContent,
        completed: li.classList.contains('completed')
      });
    });
    localStorage.setItem(STORAGE_KEY, JSON.stringify(tasks));
  }

  /**
   * Load tasks from localStorage
   */
  function loadTasks() {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (!stored) return;
      
      const tasks = JSON.parse(stored);
      tasks.forEach(function(task) {
        const taskElement = createTaskElement(task.text, task.completed);
        taskList.appendChild(taskElement);
      });
    } catch (e) {
      // Handle corrupted localStorage data gracefully
      console.warn('Failed to load tasks from localStorage:', e);
    }
  }

  /**
   * Create a task list item element
   * @param {string} text - The task text
   * @param {boolean} [completed=false] - Whether the task is completed
   * @returns {HTMLLIElement} The created task element
   */
  function createTaskElement(text, completed) {
    const li = document.createElement('li');
    
    if (completed) {
      li.classList.add('completed');
    }
    
    const textSpan = document.createElement('span');
    textSpan.className = 'task-text';
    textSpan.textContent = text;
    
    const deleteBtn = document.createElement('button');
    deleteBtn.className = 'delete-btn';
    deleteBtn.type = 'button';
    deleteBtn.textContent = '×';
    deleteBtn.setAttribute('aria-label', 'Delete task');
    
    // Delete button click handler
    deleteBtn.addEventListener('click', function(e) {
      e.stopPropagation(); // Prevent triggering completion toggle
      li.remove();
      saveTasks();
    });
    
    li.appendChild(textSpan);
    li.appendChild(deleteBtn);
    
    // Click handler to toggle completed state
    li.addEventListener('click', function() {
      li.classList.toggle('completed');
      saveTasks();
    });
    
    return li;
  }

  /**
   * Add a new task from the input field
   */
  function addTask() {
    const text = input.value.trim();
    
    // Validate - don't add empty tasks
    if (!text) {
      return;
    }
    
    const taskElement = createTaskElement(text);
    taskList.appendChild(taskElement);
    saveTasks();
    
    // Clear input and refocus for next task
    input.value = '';
    input.focus();
  }

  // Form submit handler
  form.addEventListener('submit', function(e) {
    e.preventDefault();
    addTask();
  });

  // Load tasks from localStorage on page load
  loadTasks();
})();
