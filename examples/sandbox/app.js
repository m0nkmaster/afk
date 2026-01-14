'use strict';

(function() {
  // DOM references
  const form = document.getElementById('task-form');
  const input = document.getElementById('task-input');
  const taskList = document.getElementById('task-list');

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
    });
    
    li.appendChild(textSpan);
    li.appendChild(deleteBtn);
    
    // Click handler to toggle completed state
    li.addEventListener('click', function() {
      li.classList.toggle('completed');
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
