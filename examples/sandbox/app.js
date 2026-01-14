'use strict';

(function() {
  // DOM references
  const form = document.getElementById('task-form');
  const input = document.getElementById('task-input');
  const taskList = document.getElementById('task-list');

  /**
   * Create a task list item element
   * @param {string} text - The task text
   * @returns {HTMLLIElement} The created task element
   */
  function createTaskElement(text) {
    const li = document.createElement('li');
    
    const textSpan = document.createElement('span');
    textSpan.className = 'task-text';
    textSpan.textContent = text;
    
    li.appendChild(textSpan);
    
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
    
    // Clear input and refocus for next task
    input.value = '';
    input.focus();
  }

  // Form submit handler
  form.addEventListener('submit', function(e) {
    e.preventDefault();
    addTask();
  });
})();
