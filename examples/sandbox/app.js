// Task List Application

(function() {
    'use strict';

    // DOM elements
    const taskInput = document.getElementById('new-task');
    const addButton = document.getElementById('add-task-btn');
    const taskList = document.getElementById('task-list');

    /**
     * Creates a new task list item element.
     * @param {string} text - The task text
     * @returns {HTMLLIElement} The task list item
     */
    function createTaskElement(text) {
        const li = document.createElement('li');
        li.textContent = text;
        return li;
    }

    /**
     * Adds a new task to the list.
     */
    function addTask() {
        const text = taskInput.value.trim();
        
        if (!text) {
            return;
        }

        const taskElement = createTaskElement(text);
        taskList.appendChild(taskElement);
        
        // Clear the input field
        taskInput.value = '';
        taskInput.focus();
    }

    // Event listeners
    addButton.addEventListener('click', addTask);
})();
