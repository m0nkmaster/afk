// Task List Application

(function() {
    'use strict';

    // DOM elements
    const taskForm = document.getElementById('task-form');
    const taskInput = document.getElementById('task-input');
    const taskList = document.getElementById('task-list');

    /**
     * Toggles the completed state of a task
     * @param {HTMLLIElement} taskElement - The task list item
     */
    function toggleComplete(taskElement) {
        taskElement.classList.toggle('completed');
    }

    /**
     * Creates a task list item element
     * @param {string} text - The task text
     * @returns {HTMLLIElement} The created list item
     */
    function createTaskElement(text) {
        const li = document.createElement('li');
        
        const span = document.createElement('span');
        span.textContent = text;
        span.addEventListener('click', function() {
            toggleComplete(li);
        });
        li.appendChild(span);
        
        return li;
    }

    /**
     * Adds a new task to the list
     * @param {string} text - The task text
     */
    function addTask(text) {
        const taskElement = createTaskElement(text);
        taskList.appendChild(taskElement);
    }

    /**
     * Handles form submission to add a new task
     * @param {Event} event - The submit event
     */
    function handleSubmit(event) {
        event.preventDefault();
        
        const text = taskInput.value.trim();
        if (text) {
            addTask(text);
            taskInput.value = '';
            taskInput.focus();
        }
    }

    // Event listeners
    taskForm.addEventListener('submit', handleSubmit);
})();
